use std::fmt;

use anyhow::{anyhow, bail, ensure, Context, Result};
use async_trait::async_trait;
use camino::Utf8Path;
use include_dir::{include_dir, Dir, DirEntry};
use tokio::sync::OnceCell;
use tracing::trace;

use crate::core::config::Config;
use crate::core::manifest::{ManifestDependency, Summary, TomlManifest};
use crate::core::package::{Package, PackageId};
use crate::core::source::Source;
use crate::core::SourceId;
use crate::flock::protected_run_if_not_ok;
use crate::internal::fsx;
use crate::internal::fsx::PathUtf8Ext;
use crate::sources::PathSource;

/// Serves Cairo standard library packages.
pub struct StandardLibSource<'c> {
    config: &'c Config,
    path_source: OnceCell<PathSource<'c>>,
}

impl<'c> StandardLibSource<'c> {
    pub fn new(config: &'c Config) -> Self {
        Self {
            config,
            path_source: OnceCell::new(),
        }
    }

    async fn ensure_loaded(&self) -> Result<&PathSource<'c>> {
        self.path_source.get_or_try_init(|| self.load()).await
    }

    #[tracing::instrument(name = "standard_lib_source_load", level = "trace", skip(self))]
    async fn load(&self) -> Result<PathSource<'c>> {
        static CORELIB: Dir<'_> = include_dir!("$SCARB_CORE_PATH");
        static SCARBLIB: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/scarblib");

        let tag = core_version_tag();

        let tag_fs = self
            .config
            .dirs()
            .registry_dir()
            .into_child("std")
            .into_child(&tag);

        let tag_path = tag_fs.path_existent()?;

        protected_run_if_not_ok!(&tag_fs, &self.config.package_cache_lock(), {
            trace!("extracting Cairo standard library: {tag}");
            unsafe {
                tag_fs.recreate()?;
            }

            let base_path = tag_fs.path_existent()?;

            extract_with_templating(&CORELIB, &base_path.join("core"))
                .context("failed to extract Cairo standard library (corelib)")?;
            extract_with_templating(&SCARBLIB, base_path)
                .context("failed to extract Cairo standard library (scarblib)")?;
        });

        check_corelib_version(&tag_fs.path_existent()?.join("core").join("Scarb.toml"))?;

        Ok(PathSource::recursive_at(
            tag_path,
            SourceId::for_std(),
            self.config,
        ))
    }
}

#[async_trait]
impl Source for StandardLibSource<'_> {
    #[tracing::instrument(level = "trace", skip(self))]
    async fn query(&self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
        self.ensure_loaded().await?.query(dependency).await
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn download(&self, package_id: PackageId) -> Result<Package> {
        self.ensure_loaded().await?.download(package_id).await
    }
}

impl fmt::Debug for StandardLibSource<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StandardLibSource").finish_non_exhaustive()
    }
}

fn core_version_tag() -> String {
    let core_version_info = crate::version::get().cairo;
    core_version_info
        .commit_info
        .map(|commit| {
            assert!(!commit.short_commit_hash.starts_with('v'));
            commit.short_commit_hash.to_string()
        })
        .unwrap_or_else(|| format!("v{}", core_version_info.version))
}

fn extract_with_templating(dir: &Dir<'_>, base_path: &Utf8Path) -> Result<()> {
    fsx::create_dir_all(base_path)?;

    for entry in dir.entries() {
        let path = base_path.join(entry.path().try_to_utf8()?);

        match entry {
            DirEntry::Dir(d) => {
                fsx::create_dir_all(&path)?;
                extract_with_templating(d, base_path)?;
            }
            DirEntry::File(f) => {
                let contents = f.contents();
                if path.file_name() == Some("Scarb.toml") {
                    let contents = expand_meta_variables(contents);
                    fsx::write(path, contents)?;
                } else {
                    fsx::write(path, contents)?;
                }
            }
        }
    }

    Ok(())
}

fn expand_meta_variables(contents: &[u8]) -> Vec<u8> {
    // SAFETY: We control these files, and we know that they are UTF-8.
    let contents = unsafe { std::str::from_utf8_unchecked(contents) };
    let contents = contents.replace("{{ CAIRO_VERSION }}", crate::version::get().cairo.version);
    contents.into_bytes()
}

fn check_corelib_version(core_scarb_toml: &Utf8Path) -> Result<()> {
    let comp_ver = crate::version::get().cairo.version;
    let core_ver = TomlManifest::read_from_path(core_scarb_toml)?
        .package
        .ok_or_else(|| anyhow!("could not get package section from `core` Scarb.toml"))?
        .version
        .resolve("version", || {
            bail!("the `core` package cannot inherit version from workspace")
        })?
        .to_string();
    ensure!(
        comp_ver == core_ver,
        "`core` version does not match Cairo compiler version"
    );
    Ok(())
}
