use std::str::FromStr;

use anyhow::{bail, ensure, Context, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::header::{
    HeaderMap, HeaderName, HeaderValue, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED,
};
use reqwest::{Response, StatusCode};
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::sync::OnceCell;
use tracing::{debug, trace, warn};

use scarb_ui::components::Status;

use crate::core::registry::client::{
    CreateScratchFileCallback, RegistryClient, RegistryDownload, RegistryResource,
};
use crate::core::registry::index::{IndexConfig, IndexRecords};
use crate::core::{Config, Package, PackageId, PackageName, SourceId};
use crate::flock::{FileLockGuard, Filesystem};

// TODO(mkaput): Progressbar.
// TODO(mkaput): Request timeout.

/// Remote registry served by the HTTP-based registry API.
pub struct HttpRegistryClient<'c> {
    config: &'c Config,
    index_config: IndexConfigManager<'c>,
}

enum HttpCacheKey {
    ETag(HeaderValue),
    LastModified(HeaderValue),
    None,
}

struct IndexConfigManager<'c> {
    source_id: SourceId,
    config: &'c Config,
    cache_file_name: String,
    cache_fs: Filesystem,
    cell: OnceCell<IndexConfig>,
}

impl<'c> HttpRegistryClient<'c> {
    pub fn new(source_id: SourceId, config: &'c Config) -> Result<Self> {
        Ok(Self {
            config,
            index_config: IndexConfigManager::new(source_id, config),
        })
    }
}

#[async_trait]
impl<'c> RegistryClient for HttpRegistryClient<'c> {
    async fn get_records(
        &self,
        package: PackageName,
        cache_key: Option<&str>,
    ) -> Result<RegistryResource<IndexRecords>> {
        let cache_key = HttpCacheKey::deserialize(cache_key);

        if cache_key.is_some() && !self.config.network_allowed() {
            debug!("network is not allowed, while cached record exists, using cache");
            return Ok(RegistryResource::InCache);
        }

        let index_config = self.index_config.load().await?;
        let records_url = index_config.index.expand(package.into())?;

        let response = self
            .config
            .online_http()?
            .get(records_url)
            .headers(cache_key.to_headers_for_request())
            .send()
            .await?;

        let response = match response.status() {
            StatusCode::NOT_MODIFIED => {
                ensure!(
                    cache_key.is_some(),
                    "server said not modified (HTTP 304) when no local cache exists"
                );
                return Ok(RegistryResource::InCache);
            }
            StatusCode::NOT_FOUND => {
                return Ok(RegistryResource::NotFound);
            }
            _ => response.error_for_status()?,
        };

        let cache_key = HttpCacheKey::extract(&response).serialize();

        let records = response
            .json()
            .await
            .context("failed to deserialize index records")?;

        Ok(RegistryResource::Download {
            resource: records,
            cache_key,
        })
    }

    async fn download(
        &self,
        package: PackageId,
        create_scratch_file: CreateScratchFileCallback,
    ) -> Result<RegistryDownload<FileLockGuard>> {
        let index_config = self.index_config.load().await?;
        let dl_url = index_config.dl.expand(package.into())?;

        self.config
            .ui()
            .print(Status::new("Downloading", &package.to_string()));

        let response = self.config.online_http()?.get(dl_url).send().await?;

        let response = match response.status() {
            StatusCode::NOT_MODIFIED => {
                bail!("packages archive server is not allowed to say not modified (HTTP 304)")
            }
            StatusCode::NOT_FOUND => {
                return Ok(RegistryDownload::NotFound);
            }
            _ => response.error_for_status()?,
        };

        let mut output_file = create_scratch_file(self.config)?.into_async();

        let mut stream = response.bytes_stream();
        let mut writer = BufWriter::new(&mut *output_file);
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("failed to read response chunk")?;
            io::copy_buf(&mut &*chunk, &mut writer)
                .await
                .context("failed to save response chunk on disk")?;
        }

        let output_file = output_file.into_sync().await;

        Ok(RegistryDownload::Download(output_file))
    }

    async fn supports_publish(&self) -> Result<bool> {
        // TODO(mkaput): Publishing to HTTP registries is not implemented yet.
        Ok(false)
    }

    async fn publish(&self, _package: Package, _tarball: FileLockGuard) -> Result<()> {
        todo!("Publishing to HTTP registries is not implemented yet.")
    }
}

impl HttpCacheKey {
    fn extract(response: &Response) -> Self {
        if let Some(val) = response.headers().get(ETAG) {
            Self::ETag(val.clone())
        } else if let Some(val) = response.headers().get(LAST_MODIFIED) {
            Self::LastModified(val.clone())
        } else {
            Self::None
        }
    }

    fn to_headers_for_request(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        match self {
            Self::ETag(val) => {
                headers.insert(IF_NONE_MATCH, val.clone());
            }
            Self::LastModified(val) => {
                headers.insert(IF_MODIFIED_SINCE, val.clone());
            }
            Self::None => {}
        }
        headers
    }

    fn serialize(&self) -> Option<String> {
        let (key, val) = match self {
            HttpCacheKey::ETag(val) => (ETAG, val),
            HttpCacheKey::LastModified(val) => (LAST_MODIFIED, val),
            HttpCacheKey::None => return None,
        };

        Some(format!(
            "{key}: {val}",
            val = String::from_utf8_lossy(val.as_bytes())
        ))
    }

    fn deserialize(cache_key: Option<&str>) -> Self {
        let Some(cache_key) = cache_key else {
            return Self::None;
        };
        let Some((key, value)) = cache_key.split_once(':') else {
            warn!("invalid cache key: {cache_key}");
            return Self::None;
        };
        let Ok(key) = HeaderName::from_str(key) else {
            warn!("invalid cache key: {cache_key}");
            return Self::None;
        };
        let Ok(value) = HeaderValue::from_str(value.trim()) else {
            warn!("invalid cache key: {cache_key}");
            return Self::None;
        };
        match key {
            ETAG => Self::ETag(value),
            LAST_MODIFIED => Self::LastModified(value),
            _ => {
                warn!("invalid cache key: {cache_key}");
                Self::None
            }
        }
    }

    fn is_some(&self) -> bool {
        !self.is_none()
    }

    fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl<'c> IndexConfigManager<'c> {
    fn new(source_id: SourceId, config: &'c Config) -> Self {
        let cache_file_name = format!("{}.json", source_id.ident());
        let cache_fs = config
            .dirs()
            .registry_dir()
            .into_child("configs")
            .into_child("http");

        Self {
            source_id,
            config,
            cache_file_name,
            cache_fs,
            cell: OnceCell::new(),
        }
    }

    async fn load(&self) -> Result<&IndexConfig> {
        self.cell
            .get_or_try_init(|| self.load_impl_with_log())
            .await
            .context("failed to fetch registry config")
    }

    async fn load_impl_with_log(&self) -> Result<IndexConfig> {
        let index_config = self.fetch().await?;
        trace!(index_config = %serde_json::to_string(&index_config).unwrap());
        Ok(index_config)
    }

    async fn fetch(&self) -> Result<IndexConfig> {
        match self.fetch_from_cache().await {
            Ok(Some(index_config)) => {
                debug!("using cached config");
                return Ok(index_config);
            }
            Ok(None) => {}
            Err(err) => {
                warn!("failed to fetch cached config: {err:?}");
            }
        }

        let index_config = self.fetch_from_origin().await?;

        if let Err(err) = self.save_in_cache(&index_config).await {
            warn!("failed to save config in cache: {err:?}");
        }

        Ok(index_config)
    }

    fn may_be_cached(&self) -> bool {
        self.cache_fs
            .path_unchecked()
            .join(&self.cache_file_name)
            .exists()
    }

    async fn fetch_from_cache(&self) -> Result<Option<IndexConfig>> {
        if !self.may_be_cached() {
            return Ok(None);
        }
        let mut file = self
            .cache_fs
            .open_ro(&self.cache_file_name, &self.cache_file_name, self.config)?
            .into_async();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;
        let index_config = serde_json::from_slice(&buffer)?;
        Ok(Some(index_config))
    }

    async fn save_in_cache(&self, index_config: &IndexConfig) -> Result<()> {
        let mut file = self
            .cache_fs
            .open_rw(&self.cache_file_name, &self.cache_file_name, self.config)?
            .into_async();
        let json = serde_json::to_vec(index_config)?;
        file.write_all(&json).await?;
        Ok(())
    }

    async fn fetch_from_origin(&self) -> Result<IndexConfig> {
        let index_config_url = self
            .source_id
            .url
            .join(IndexConfig::WELL_KNOWN_PATH)
            .expect("Registry config URL should always be valid.");
        debug!("fetching registry config: {index_config_url}");

        let index_config = self
            .config
            .online_http()?
            .get(index_config_url)
            .send()
            .await?
            .error_for_status()?
            .json::<IndexConfig>()
            .await?;

        Ok(index_config)
    }
}
