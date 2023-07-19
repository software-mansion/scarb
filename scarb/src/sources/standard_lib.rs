use std::fmt;

use anyhow::{Context, Result};
use async_trait::async_trait;
use camino::Utf8Path;
use include_dir::{include_dir, Dir, DirEntry};
use tokio::sync::OnceCell;
use tracing::trace;

use crate::core::config::Config;
use crate::core::manifest::{ManifestDependency, Summary};
use crate::core::package::{Package, PackageId};
use crate::core::source::Source;
use crate::core::SourceId;
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

        let registry_fs = self.config.dirs().registry_dir();
        let std_fs = registry_fs.child("std");
        let tag_fs = std_fs.child(&tag);
        let tag_path = tag_fs.path_existent()?;

        // The following sequence of if statements & advisory locks implements a file system-based
        // mutex, that synchronizes extraction logic. The first condition checks if extraction has
        // happened in the past. If not, then we acquire the advisory lock (which means waiting for
        // our time slice to do the job). Successful lock acquisition does not mean though that we
        // still have to perform the extraction! While we waited for our time slice, another process
        // could just do the extraction! The second condition prevents repeating the work.
        //
        // This is actually very important for correctness. The another process that performed
        // the extraction, will highly probably soon try to read the extracted files. If we recreate
        // the filesystem now, we will cause that process to crash. That's what happened on Windows
        // in examples tests, when the second condition was missing.
        if !tag_fs.is_ok() {
            let _lock = self.config.package_cache_lock().acquire_async().await?;
            if !tag_fs.is_ok() {
                trace!("extracting Cairo standard library: {tag}");
                unsafe {
                    tag_fs.recreate()?;
                }

                let base_path = tag_fs.path_existent()?;

                extract_with_templating(&CORELIB, &base_path.join("core"))
                    .context("failed to extract Cairo standard library (corelib)")?;
                extract_with_templating(&SCARBLIB, base_path)
                    .context("failed to extract Cairo standard library (scarblib)")?;

                tag_fs.mark_ok()?;
            }
        }

        Ok(PathSource::recursive_at(
            tag_path,
            SourceId::for_std(),
            self.config,
        ))
    }
}

#[async_trait]
impl<'c> Source for StandardLibSource<'c> {
    #[tracing::instrument(level = "trace", skip(self))]
    async fn query(&self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
        self.ensure_loaded().await?.query(dependency).await
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn download(&self, package_id: PackageId) -> Result<Package> {
        self.ensure_loaded().await?.download(package_id).await
    }
}

impl<'c> fmt::Debug for StandardLibSource<'c> {
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
