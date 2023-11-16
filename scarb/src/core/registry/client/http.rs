use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{ensure, Context, Result};
use async_trait::async_trait;
use fs4::tokio::AsyncFileExt;
use futures::StreamExt;
use reqwest::header::{
    HeaderMap, HeaderName, HeaderValue, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED,
};
use reqwest::{Response, StatusCode};
use scarb_ui::components::Status;
use tokio::fs::OpenOptions;
use tokio::io;
use tokio::io::BufWriter;
use tokio::sync::OnceCell;
use tracing::{debug, error, trace, warn};

use crate::core::registry::client::{RegistryClient, RegistryResource};
use crate::core::registry::index::{IndexConfig, IndexRecords};
use crate::core::{Config, Package, PackageId, PackageName, SourceId};
use crate::flock::{FileLockGuard, Filesystem};

// TODO(mkaput): Progressbar.
// TODO(mkaput): Request timeout.

/// Remote registry served by the HTTP-based registry API.
pub struct HttpRegistryClient<'c> {
    source_id: SourceId,
    config: &'c Config,
    cached_index_config: OnceCell<IndexConfig>,
    dl_fs: Filesystem<'c>,
}

enum HttpCacheKey {
    ETag(HeaderValue),
    LastModified(HeaderValue),
    None,
}

impl<'c> HttpRegistryClient<'c> {
    pub fn new(source_id: SourceId, config: &'c Config) -> Result<Self> {
        let dl_fs = config
            .dirs()
            .registry_dir()
            .into_child("dl")
            .into_child(source_id.ident());

        Ok(Self {
            source_id,
            config,
            cached_index_config: Default::default(),
            dl_fs,
        })
    }

    async fn index_config(&self) -> Result<&IndexConfig> {
        // TODO(mkaput): Cache config locally, honouring ETag and Last-Modified headers.

        async fn load(source_id: SourceId, config: &Config) -> Result<IndexConfig> {
            let index_config_url = source_id
                .url
                .join(IndexConfig::WELL_KNOWN_PATH)
                .expect("Registry config URL should always be valid.");
            debug!("fetching registry config: {index_config_url}");

            let index_config = config
                .online_http()?
                .get(index_config_url)
                .send()
                .await?
                .error_for_status()?
                .json::<IndexConfig>()
                .await?;

            trace!(index_config = %serde_json::to_string(&index_config).unwrap());

            Ok(index_config)
        }

        self.cached_index_config
            .get_or_try_init(|| load(self.source_id, self.config))
            .await
            .context("failed to fetch registry config")
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

        let index_config = self.index_config().await?;
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

    async fn download(&self, package: PackageId) -> Result<RegistryResource<PathBuf>> {
        let dl_url = self.index_config().await?.dl.expand(package.into())?;

        self.config
            .ui()
            .print(Status::new("Downloading", &package.to_string()));

        let response = self
            .config
            .online_http()?
            .get(dl_url)
            .send()
            .await?
            .error_for_status();

        if let Err(err) = &response {
            if let Some(status) = err.status() {
                if status == StatusCode::NOT_FOUND {
                    error!("package `{package}` not found in registry: {err:?}");
                    return Ok(RegistryResource::NotFound);
                }
            }
        }

        let response = response?;

        let output_path = self.dl_fs.path_existent()?.join(package.tarball_name());
        let output_file = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .create(true)
            .open(&output_path)
            .await
            .with_context(|| format!("failed to open: {output_path}"))?;

        output_file
            .lock_exclusive()
            .with_context(|| format!("failed to lock file: {output_path}"))?;

        let mut stream = response.bytes_stream();
        let mut writer = BufWriter::new(output_file);
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("failed to read response chunk")?;
            io::copy_buf(&mut &*chunk, &mut writer)
                .await
                .context("failed to save response chunk on disk")?;
        }

        Ok(RegistryResource::Download {
            resource: output_path.into_std_path_buf(),
            cache_key: None,
        })
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
