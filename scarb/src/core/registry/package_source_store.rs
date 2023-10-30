use std::io::{Seek, SeekFrom};
use std::ops::DerefMut;

use anyhow::{ensure, Context, Result};
use camino::Utf8PathBuf;
use tokio::task::spawn_blocking;
use tracing::{debug, trace};

use crate::core::{Config, PackageId, SourceId};
use crate::flock::{protected_run_if_not_ok, FileLockGuard, Filesystem, OK_FILE};
use crate::internal::fsx::PathUtf8Ext;
use crate::internal::restricted_names::is_windows_restricted_path;

pub struct PackageSourceStore<'a> {
    fs: Filesystem,
    config: &'a Config,
}

impl<'a> PackageSourceStore<'a> {
    pub fn new(source: SourceId, config: &'a Config) -> Self {
        let fs = config
            .dirs()
            .registry_dir()
            .into_child("src")
            .into_child(source.ident());
        Self { fs, config }
    }

    /// Extract a downloaded package archive into a location where it is ready to be compiled.
    ///
    /// No action is taken if the source looks like it's already unpacked.
    ///
    /// This function takes the archive by ownership for implementation simplicity.
    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn extract(&self, pkg: PackageId, archive: FileLockGuard) -> Result<Utf8PathBuf> {
        trace!("attempting to extract `{pkg}`");
        trace!(archive = ?archive.path());
        self.extract_impl(pkg, archive)
            .await
            .with_context(|| format!("failed to extract: {pkg}"))
    }

    async fn extract_impl(
        &self,
        pkg: PackageId,
        mut archive: FileLockGuard,
    ) -> Result<Utf8PathBuf> {
        let prefix = pkg.tarball_basename();
        let fs = self.fs.child(&prefix);
        let parent_path = self.fs.path_existent()?.to_owned();
        let output_path = fs.path_existent()?.to_owned();
        trace!(?output_path);

        assert_eq!(parent_path.join(&prefix), output_path);

        protected_run_if_not_ok!(&fs, &self.config.package_cache_lock(), {
            debug!("starting extraction");

            // Wipe anything already extracted.
            unsafe {
                fs.recreate()?;
            }

            spawn_blocking(move || -> Result<()> {
                // FIXME(mkaput): Verify VERSION is 1.

                let mut tar = {
                    archive.seek(SeekFrom::Start(0))?;
                    let zst = zstd::Decoder::new(archive.deref_mut())?;
                    // FIXME(mkaput): Protect against zip bomb attacks (https://github.com/rust-lang/cargo/pull/11337).
                    // FIXME(mkaput): Protect against CVE-2023-38497 (https://github.com/rust-lang/cargo/pull/12443).
                    tar::Archive::new(zst)
                };

                for entry in tar.entries()? {
                    let mut entry = entry.with_context(|| "failed to iterate over archive")?;
                    let entry_path = entry
                        .path()
                        .with_context(|| "failed to read entry path")?
                        .try_to_utf8()?;

                    // Ensure extracting will not accidentally or maliciously overwrite files
                    // outside extraction directory.
                    ensure!(
                        entry_path.starts_with(&prefix),
                        "invalid package tarball, contains a file {entry_path} \
                        which is not under {prefix}"
                    );

                    // Prevent unpacking OK-file.
                    if entry_path.file_name().unwrap_or_default() == OK_FILE {
                        continue;
                    }

                    let mut r = entry.unpack_in(&parent_path).map_err(anyhow::Error::from);

                    if cfg!(windows) && is_windows_restricted_path(entry_path.as_std_path()) {
                        r = r.context("path contains Windows restricted file name");
                    }

                    r.with_context(|| format!("failed to extract: {entry_path}"))?;
                }

                Ok(())
            })
            .await??;
        });

        trace!("extraction succeeded");
        Ok(output_path)
    }
}
