use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{ensure, Result};
use async_trait::async_trait;
use tokio::task::spawn_blocking;
use url::Url;

use crate::core::registry::client::RegistryClient;
use crate::core::registry::index::{IndexRecords, TemplateUrl};
use crate::core::{PackageId, PackageName};
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
pub struct LocalRegistryClient {
    index_template_url: TemplateUrl,
    dl_template_url: TemplateUrl,
}

impl LocalRegistryClient {
    pub fn new(root: &Path) -> Result<Self> {
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
        })
    }
}

#[async_trait]
impl RegistryClient for LocalRegistryClient {
    fn is_offline(&self) -> bool {
        true
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn get_records(&self, package: PackageName) -> Result<Option<Arc<IndexRecords>>> {
        let records_path = self
            .index_template_url
            .expand(package.into())?
            .to_file_path()
            .expect("Local index should always use file:// URLs.");

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
        Ok(self
            .dl_template_url
            .expand(package.into())?
            .to_file_path()
            .expect("Local index should always use file:// URLs."))
    }
}
