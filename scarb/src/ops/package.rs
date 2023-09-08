use std::collections::BTreeMap;

use anyhow::{ensure, Result};
use camino::Utf8PathBuf;

use scarb_ui::components::Status;

use crate::core::{Package, PackageId, PackageName, Workspace};
use crate::flock::FileLockGuard;
use crate::{ops, DEFAULT_SOURCE_PATH, MANIFEST_FILE_NAME};

const VERSION: u8 = 1;
const VERSION_FILE_NAME: &str = "VERSION";
const ORIGINAL_MANIFEST_FILE_NAME: &str = "Scarb.orig.toml";

const RESERVED_FILES: &[&str] = &[VERSION_FILE_NAME, ORIGINAL_MANIFEST_FILE_NAME];

pub struct PackageOpts;

/// A listing of files to include in the archive, without actually building it yet.
///
/// This struct is used to facilitate both building the package, and listing its contents without
/// actually making it.
type ArchiveRecipe<'a> = Vec<ArchiveFile<'a>>;

struct ArchiveFile<'a> {
    /// The relative path in the archive (not including top-level package name directory).
    path: Utf8PathBuf,
    #[allow(dead_code)]
    /// The contents of the file.
    contents: ArchiveFileContents<'a>,
}

enum ArchiveFileContents<'a> {
    /// Absolute path to the file on disk to add to the archive.
    OnDisk(Utf8PathBuf),

    /// Generate file contents automatically.
    ///
    /// This variant stores a closure, so that file generation can be deferred to the very moment
    /// it is needed.
    /// For example, when listing package contents, we do not have files contents.
    Generated(Box<dyn FnOnce() -> Result<Vec<u8>> + 'a>),
}

pub fn package(
    packages: &[PackageId],
    opts: &PackageOpts,
    ws: &Workspace<'_>,
) -> Result<Vec<FileLockGuard>> {
    before_package(ws)?;

    packages
        .iter()
        .map(|pkg| {
            let pkg_name = pkg.to_string();
            let message = Status::new("Packaging", &pkg_name);
            if packages.len() <= 1 {
                ws.config().ui().verbose(message);
            } else {
                ws.config().ui().print(message);
            }

            package_one_impl(*pkg, opts, ws)
        })
        .collect()
}

pub fn package_one(
    package: PackageId,
    opts: &PackageOpts,
    ws: &Workspace<'_>,
) -> Result<FileLockGuard> {
    before_package(ws)?;
    package_one_impl(package, opts, ws)
}

pub fn package_list(
    packages: &[PackageId],
    opts: &PackageOpts,
    ws: &Workspace<'_>,
) -> Result<BTreeMap<PackageName, Vec<Utf8PathBuf>>> {
    packages
        .iter()
        .map(|pkg| Ok((pkg.name.clone(), list_one_impl(*pkg, opts, ws)?)))
        .collect()
}

fn before_package(ws: &Workspace<'_>) -> Result<()> {
    ops::resolve_workspace(ws)?;
    Ok(())
}

fn package_one_impl(
    pkg_id: PackageId,
    _opts: &PackageOpts,
    ws: &Workspace<'_>,
) -> Result<FileLockGuard> {
    let pkg = ws.fetch_package(&pkg_id)?;

    // TODO(mkaput): Check metadata

    // TODO(#643): Check dirty in VCS (but do not do it when listing!).

    let _recipe = prepare_archive_recipe(pkg, ws)?;

    todo!("Actual packaging is not implemented yet.")
}

fn list_one_impl(
    pkg_id: PackageId,
    _opts: &PackageOpts,
    ws: &Workspace<'_>,
) -> Result<Vec<Utf8PathBuf>> {
    let pkg = ws.fetch_package(&pkg_id)?;
    let recipe = prepare_archive_recipe(pkg, ws)?;
    Ok(recipe.into_iter().map(|f| f.path).collect())
}

fn prepare_archive_recipe<'a>(
    pkg: &'a Package,
    ws: &'a Workspace<'_>,
) -> Result<ArchiveRecipe<'a>> {
    let mut recipe = source_files(pkg)?;

    check_no_reserved_files(&recipe)?;

    // Add normalized manifest file.
    recipe.push(ArchiveFile {
        path: MANIFEST_FILE_NAME.into(),
        contents: ArchiveFileContents::Generated(Box::new(|| normalize_manifest(pkg, ws))),
    });

    // Add original manifest file.
    recipe.push(ArchiveFile {
        path: ORIGINAL_MANIFEST_FILE_NAME.into(),
        contents: ArchiveFileContents::OnDisk(pkg.manifest_path().to_owned()),
    });

    // Add archive version file.
    recipe.push(ArchiveFile {
        path: VERSION_FILE_NAME.into(),
        contents: ArchiveFileContents::Generated(Box::new(|| Ok(VERSION.to_string().into_bytes()))),
    });

    // Sort archive files alphabetically, putting the version file first.
    recipe.sort_unstable_by_key(|f| {
        let priority = if f.path == VERSION_FILE_NAME { 0 } else { 1 };
        (priority, f.path.clone())
    });

    Ok(recipe)
}

fn source_files(pkg: &Package) -> Result<ArchiveRecipe<'_>> {
    // TODO(mkaput): Implement this properly.
    let mut recipe = vec![ArchiveFile {
        path: DEFAULT_SOURCE_PATH.into(),
        contents: ArchiveFileContents::OnDisk(pkg.root().join(DEFAULT_SOURCE_PATH)),
    }];

    // Add reserved files if they exist in source. They will be rejected later on.
    for &file in RESERVED_FILES {
        let path = pkg.root().join(file);
        if path.exists() {
            recipe.push(ArchiveFile {
                path: file.into(),
                contents: ArchiveFileContents::OnDisk(path),
            });
        }
    }

    Ok(recipe)
}

fn check_no_reserved_files(recipe: &ArchiveRecipe<'_>) -> Result<()> {
    let mut found = Vec::new();
    for file in recipe {
        if RESERVED_FILES.contains(&file.path.as_str()) {
            found.push(file.path.as_str());
        }
    }
    ensure!(
        found.is_empty(),
        "invalid inclusion of reserved files in package: {}",
        found.join(", ")
    );
    Ok(())
}

fn normalize_manifest(_pkg: &Package, _ws: &Workspace<'_>) -> Result<Vec<u8>> {
    // TODO(mkaput): Implement this properly.
    Ok("[package]".to_string().into_bytes())
}
