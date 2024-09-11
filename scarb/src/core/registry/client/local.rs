use std::fs::OpenOptions;
use std::io;
use std::io::{BufReader, BufWriter, Seek, SeekFrom};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Error, Result};
use async_trait::async_trait;
use fs4::FileExt;
use tokio::task::spawn_blocking;
use tracing::trace;
use url::Url;

use crate::core::registry::client::{
    CreateScratchFileCallback, RegistryClient, RegistryDownload, RegistryResource, RegistryUpload,
};
use crate::core::registry::index::{IndexDependency, IndexRecord, IndexRecords, TemplateUrl};
use crate::core::{Checksum, Config, Digest, Package, PackageId, PackageName, Summary};
use crate::flock::{FileLockGuard, Filesystem};
use crate::internal::fsx;
use crate::internal::fsx::PathBufUtf8Ext;

/// Local registry that lives on the filesystem as a set of `.tar.zst` files with an `index`
/// directory in the standard registry index format.
///
/// ## Filesystem hierarchy
///
/// Here is an example layout of a local registry on a local filesystem:
///
/// ```text
/// [registry root]/
/// ├── index/                              # registry index
/// │  ├── al/
/// │  │  └── ex/
/// │  │     ├── alexandria_ascii.json
/// │  │     └── alexandria_math.json
/// │  ├── ca/
/// │  │  └── ir/
/// │  │     └── cairo_lib.json
/// │  └── op/
/// │     └── en/
/// │        └── open_zeppelin.json
/// ├── alexandria_ascii-0.1.0.tar.zst      # pre-downloaded package tarballs
/// ├── alexandria_math-0.1.0.tar.zst
/// ├── cairo_lib-0.2.0.tar.zst
/// └── open_zeppelin-0.7.0.tar.zst
/// ```
pub struct LocalRegistryClient<'c> {
    index_template_url: TemplateUrl,
    dl_template_url: TemplateUrl,
    config: &'c Config,
}

impl<'c> LocalRegistryClient<'c> {
    pub fn new(root: &Path, config: &'c Config) -> Result<Self> {
        // NOTE: If we'd put this check after canonicalization, the latter would fail with IO error
        // on Linux, making this logic non-deterministic from tests point of view.
        ensure!(
            root.is_dir(),
            "local registry path is not a directory: {}",
            root.display()
        );

        let root = fsx::canonicalize(root)?;

        let root_url = Url::from_directory_path(root)
            .expect("Canonical path should always be convertible to URL.");

        let index_template_url =
            TemplateUrl::new(&format!("{root_url}index/{{prefix}}/{{package}}.json"));

        let dl_template_url =
            TemplateUrl::new(&format!("{root_url}{{package}}-{{version}}.tar.zst"));

        Ok(Self {
            index_template_url,
            dl_template_url,
            config,
        })
    }

    fn records_path(&self, package: &PackageName) -> PathBuf {
        self.index_template_url
            .expand(package.into())
            .unwrap()
            .to_file_path()
            .unwrap()
    }

    fn dl_path(&self, package: PackageId) -> PathBuf {
        self.dl_template_url
            .expand(package.into())
            .unwrap()
            .to_file_path()
            .unwrap()
    }
}

#[async_trait]
impl RegistryClient for LocalRegistryClient<'_> {
    #[tracing::instrument(level = "trace", skip_all)]
    async fn get_records(
        &self,
        package: PackageName,
        cache_key: Option<&str>,
    ) -> Result<RegistryResource<IndexRecords>> {
        trace!(?package);

        assert!(cache_key.is_none());

        let records_path = self.records_path(&package);

        spawn_blocking(move || {
            let records = match fsx::read(records_path) {
                Err(e)
                    if e.downcast_ref::<io::Error>()
                        .map_or(false, |ioe| ioe.kind() == io::ErrorKind::NotFound) =>
                {
                    return Ok(RegistryResource::NotFound);
                }
                r => r?,
            };
            let records = serde_json::from_slice(&records)?;
            Ok(RegistryResource::Download {
                resource: records,
                cache_key: None,
            })
        })
        .await?
    }

    async fn download(
        &self,
        package: PackageId,
        _: CreateScratchFileCallback,
    ) -> Result<RegistryDownload<FileLockGuard>> {
        let dl_path = self.dl_path(package).try_into_utf8()?;
        let base_path = dl_path
            .parent()
            .expect("Parent directory should always exist.")
            .to_owned();
        let file_name = dl_path.file_name().expect("File name should always exist.");

        let fs = Filesystem::new(base_path);
        let file = fs.open_ro(file_name, file_name, self.config)?;

        Ok(RegistryDownload::Download(file))
    }

    async fn supports_publish(&self) -> Result<bool> {
        Ok(true)
    }

    async fn publish(&self, package: Package, tarball: FileLockGuard) -> Result<RegistryUpload> {
        let summary = package.manifest.summary.clone();
        let records_path = self.records_path(&summary.package_id.name);
        let dl_path = self.dl_path(summary.package_id);

        spawn_blocking(move || publish_impl(summary, tarball, records_path, dl_path))
            .await
            .with_context(|| format!("failed to publish package: {package}"))?
    }
}

fn publish_impl(
    summary: Summary,
    tarball: FileLockGuard,
    records_path: PathBuf,
    dl_path: PathBuf,
) -> Result<RegistryUpload, Error> {
    let checksum = Digest::recommended().update_read(tarball.deref())?.finish();
    let tarball_path = tarball.path().to_owned();

    // Drop the FileLockGuard to release the tarball file RW lock, otherwise the package cannot be copied to local registry on Windows.
    drop(tarball);
    fsx::copy(tarball_path, dl_path)?;

    let record = build_record(summary, checksum);

    edit_records(&records_path, move |records| {
        // Remove existing record if exists (note: version is the key).
        if let Some(idx) = records.iter().position(|r| r.version == record.version) {
            records.swap_remove(idx);
        }

        records.push(record);

        records.sort_by_cached_key(|r| r.version.clone());
    })
    .with_context(|| format!("failed to edit records file: {}", records_path.display()))?;

    Ok(RegistryUpload::Success)
}

fn build_record(summary: Summary, checksum: Checksum) -> IndexRecord {
    IndexRecord {
        version: summary.package_id.version.clone(),
        dependencies: summary
            .publish_dependencies()
            .map(|dep| IndexDependency {
                name: dep.name.clone(),
                req: dep.version_req.clone().into(),
            })
            .collect(),
        checksum,
        no_core: summary.no_core,
    }
}

fn edit_records(records_path: &Path, func: impl FnOnce(&mut IndexRecords)) -> Result<()> {
    fsx::create_dir_all(records_path.parent().unwrap())?;
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(records_path)
        .context("failed to open file")?;

    file.lock_exclusive()
        .context("failed to acquire exclusive file access")?;

    let is_empty = file.metadata().context("failed to read metadata")?.len() == 0;

    let mut records: IndexRecords = if !is_empty {
        let file = BufReader::new(&file);
        serde_json::from_reader(file).context("failed to deserialize file contents")?
    } else {
        IndexRecords::new()
    };

    func(&mut records);

    {
        file.seek(SeekFrom::Start(0))
            .with_context(|| "failed to seek file cursor".to_string())?;
        file.set_len(0)
            .with_context(|| "failed to truncate file".to_string())?;

        let file = BufWriter::new(file);
        serde_json::to_writer(file, &records).context("failed to serialize file")?;
    }

    Ok(())
}
