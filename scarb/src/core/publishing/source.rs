use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use ignore::{DirEntry, WalkBuilder};

use crate::core::Package;
use crate::internal::fsx::PathBufUtf8Ext;
use crate::{
    CAIRO_PROJECT_FILE_NAME, CARGO_LOCKFILE_FILE_NAME, CARGO_MANIFEST_FILE_NAME,
    DEFAULT_TARGET_DIR_NAME, LOCK_FILE_NAME, MANIFEST_FILE_NAME, SCARB_IGNORE_FILE_NAME,
};

/// List all files relevant to building this package inside this source.
///
/// The basic assumption is that all files in the package directory are relevant for building this
/// package, provided that they potentially can be committed to the source directory. The following
/// rules hold:
/// * Look for any `.scarbignore`, `.gitignore` or `.ignore`-like files, using the [`ignore`] crate.
/// * Skip `.git` directory.
/// * Skip any subdirectories containing `Scarb.toml`.
/// * Skip `<root>/target` directory.
/// * Skip `Scarb.lock` file.
/// * Skip README and LICENSE files.
/// * **Skip `Scarb.toml` file, as users of this function may want to generate it themselves.**
/// * Symlinks within the package directory are followed, while symlinks outside are just skipped.
/// * Avoid crossing file system boundaries, because it can complicate our lives.
pub fn list_source_files(pkg: &Package) -> Result<Vec<Utf8PathBuf>> {
    let mut ret = Vec::new();
    push_worktree_files(pkg, &mut ret)
        .with_context(|| format!("failed to list source files in: {}", pkg.root()))?;
    Ok(ret)
}

fn push_worktree_files(pkg: &Package, ret: &mut Vec<Utf8PathBuf>) -> Result<()> {
    let filter = {
        let pkg = pkg.clone();
        let readme = pkg.manifest.metadata.readme.clone().unwrap_or_default();
        let license_file = pkg
            .manifest
            .metadata
            .license_file
            .clone()
            .unwrap_or_default();

        move |entry: &DirEntry| -> bool {
            let path = entry.path();
            let is_root = entry.depth() == 0;

            // Ignore symlinks pointing outside the package directory.
            if path.strip_prefix(pkg.root()).is_err() {
                return false;
            };

            // Skip any subdirectories containing `Scarb.toml`.
            if !is_root && path.join(MANIFEST_FILE_NAME).exists() {
                return false;
            }

            // Skip `Scarb.toml`, `Scarb.lock`, 'Cargo.toml`, 'Cargo.lock', `cairo_project.toml`,
            // and `target` directory.
            if entry.depth() == 1
                && ({
                    let f = entry.file_name();
                    f == MANIFEST_FILE_NAME
                        || f == LOCK_FILE_NAME
                        || f == CARGO_MANIFEST_FILE_NAME
                        || f == CARGO_LOCKFILE_FILE_NAME
                        || f == DEFAULT_TARGET_DIR_NAME
                        || f == CAIRO_PROJECT_FILE_NAME
                })
            {
                return false;
            }

            // Skip README and LICENSE files
            if path == readme || path == license_file {
                return false;
            }

            true
        }
    };
    let mut builder = WalkBuilder::new(pkg.root());
    for path in pkg.include()? {
        builder.add(&path);
    }
    builder
        .follow_links(true)
        .standard_filters(true)
        .parents(false)
        .require_git(true)
        .same_file_system(true)
        .add_custom_ignore_filename(SCARB_IGNORE_FILE_NAME)
        .filter_entry(filter)
        .build()
        .try_for_each(|entry| {
            let entry = entry?;
            if !is_dir(&entry) {
                ret.push(entry.into_path().try_into_utf8()?);
            }
            Ok(())
        })
}

fn is_dir(entry: &DirEntry) -> bool {
    entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
}
