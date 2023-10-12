use std::fs::OpenOptions;
use std::io;
use std::io::{BufReader, BufWriter, Seek, SeekFrom};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Error, Result};
use async_trait::async_trait;
use fs4::FileExt;
use tokio::task::spawn_blocking;
use url::Url;

use crate::core::registry::client::RegistryClient;
use crate::core::registry::index::{IndexDependency, IndexRecord, IndexRecords, TemplateUrl};
use crate::core::{Checksum, Digest, Package, PackageId, PackageName, Summary};
use crate::flock::FileLockGuard;
use crate::internal::fsx;

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
/// │  │     ├── alexandria_ascii
/// │  │     └── alexandria_math
/// │  ├── ca/
/// │  │  └── ir/
/// │  │     └── cairo_lib
/// │  └── op/
/// │     └── en/
/// │        └── open_zeppelin
/// ├── alexandria_ascii-0.1.0.tar.zst      # pre-downloaded package tarballs
/// ├── alexandria_math-0.1.0.tar.zst
/// ├── cairo_lib-0.2.0.tar.zst
/// └── open_zeppelin-0.7.0.tar.zst
/// ```
pub struct LocalRegistryClient {
    index_template_url: TemplateUrl,
    dl_template_url: TemplateUrl,
}

impl LocalRegistryClient {
    pub fn new(root: &Path) -> Result<Self> {
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
impl RegistryClient for LocalRegistryClient {
    fn is_offline(&self) -> bool {
        true
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn get_records(&self, package: PackageName) -> Result<Option<Arc<IndexRecords>>> {
        let records_path = self.records_path(&package);

        spawn_blocking(move || {
            let records = match fsx::read(records_path) {
                Err(e)
                    if e.downcast_ref::<io::Error>()
                        .map_or(false, |ioe| ioe.kind() == io::ErrorKind::NotFound) =>
                {
                    return Ok(None);
                }
                r => r?,
            };
            let records = serde_json::from_slice(&records)?;
            Ok(Some(Arc::new(records)))
        })
        .await?
    }

    async fn is_downloaded(&self, _package: PackageId) -> bool {
        true
    }

    async fn download(&self, package: PackageId) -> Result<PathBuf> {
        Ok(self.dl_path(package))
    }

    async fn supports_publish(&self) -> Result<bool> {
        Ok(true)
    }

    async fn publish(&self, package: Package, tarball: FileLockGuard) -> Result<()> {
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
) -> Result<(), Error> {
    fsx::copy(tarball.path(), dl_path)?;

    let checksum = Digest::recommended().update_read(tarball.deref())?.finish();

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

    Ok(())
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
