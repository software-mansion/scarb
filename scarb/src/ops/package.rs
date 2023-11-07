use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};

use anyhow::{bail, ensure, Context, Result};
use camino::Utf8PathBuf;
use indoc::{indoc, writedoc};

use crate::sources::client::PackageRepository;

use scarb_ui::components::Status;
use scarb_ui::{HumanBytes, HumanCount};
use serde::Serialize;

use crate::core::publishing::manifest_normalization::prepare_manifest_for_publish;
use crate::core::publishing::source::list_source_files;
use crate::core::{Package, PackageId, PackageName, Workspace};
use crate::flock::FileLockGuard;
use crate::internal::restricted_names;
use crate::{ops, MANIFEST_FILE_NAME, VCS_INFO_FILE_NAME};

const VERSION: u8 = 1;
const VERSION_FILE_NAME: &str = "VERSION";
const ORIGINAL_MANIFEST_FILE_NAME: &str = "Scarb.orig.toml";

const RESERVED_FILES: &[&str] = &[
    VERSION_FILE_NAME,
    ORIGINAL_MANIFEST_FILE_NAME,
    VCS_INFO_FILE_NAME,
];

#[derive(Clone)]
pub struct PackageOpts {
    pub allow_dirty: bool,
}

/// A listing of files to include in the archive, without actually building it yet.
///
/// This struct is used to facilitate both building the package, and listing its contents without
/// actually making it.
type ArchiveRecipe = Vec<ArchiveFile>;

struct ArchiveFile {
    /// The relative path in the archive (not including top-level package name directory).
    path: Utf8PathBuf,
    /// The contents of the file.
    contents: ArchiveFileContents,
}

enum ArchiveFileContents {
    /// Absolute path to the file on disk to add to the archive.
    OnDisk(Utf8PathBuf),

    /// Generate file contents automatically.
    ///
    /// This variant stores a closure, so that file generation can be deferred to the very moment
    /// it is needed.
    /// For example, when listing package contents, we do not have files contents.
    Generated(Box<dyn FnOnce() -> Result<Vec<u8>>>),
}

#[tracing::instrument(level = "debug", skip(opts, ws))]
pub fn package(
    packages: &[PackageId],
    opts: &PackageOpts,
    ws: &Workspace<'_>,
) -> Result<Vec<FileLockGuard>> {
    before_package(ws)?;

    packages
        .iter()
        .map(|pkg| package_one_impl(*pkg, opts, ws))
        .collect()
}

pub fn package_one(
    package_id: PackageId,
    opts: &PackageOpts,
    ws: &Workspace<'_>,
) -> Result<FileLockGuard> {
    package(&[package_id], opts, ws).map(|mut v| v.pop().unwrap())
}

#[tracing::instrument(level = "debug", skip(opts, ws))]
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

#[derive(Serialize)]
struct GitVcsInfo {
    sha1: String,
}

#[derive(Serialize)]
struct VcsInfo {
    git: GitVcsInfo,
    path_in_vcs: String,
}

fn extract_vcs_info(repo: PackageRepository, opts: &PackageOpts) -> Result<Option<VcsInfo>> {
    ensure!(
        opts.allow_dirty || repo.is_clean()?,
        indoc! {r#"
            cannot package a repository containing uncommited changes
            help: to proceed despite this and include the uncommitted changes, pass the `--allow-dirty` flag
        "#}
    );

    Ok(Some(VcsInfo {
        path_in_vcs: repo.path_in_vcs()?,
        git: GitVcsInfo {
            sha1: repo.head_rev_hash()?,
        },
    }))
}

#[tracing::instrument(level = "trace", skip(opts, ws))]
fn package_one_impl(
    pkg_id: PackageId,
    opts: &PackageOpts,
    ws: &Workspace<'_>,
) -> Result<FileLockGuard> {
    let pkg = ws.fetch_package(&pkg_id)?;

    ws.config()
        .ui()
        .print(Status::new("Packaging", &pkg_id.to_string()));

    // TODO(mkaput): Check metadata

    let recipe = prepare_archive_recipe(pkg, opts)?;
    let num_files = recipe.len();

    // Package up and test a temporary tarball and only move it to the final location if it actually
    // passes all verification checks. Any previously existing tarball can be assumed as corrupt
    // or invalid, so we can overwrite it if it exists.
    let filename = pkg_id.tarball_name();
    let target_dir = ws.target_dir().child("package");

    let mut dst =
        target_dir.open_rw(format!(".{filename}"), "package scratch space", ws.config())?;

    dst.set_len(0)
        .with_context(|| format!("failed to truncate: {filename}"))?;

    let uncompressed_size = tar(pkg_id, recipe, &mut dst, ws)?;

    // TODO(mkaput): Verify.

    dst.seek(SeekFrom::Start(0))?;

    dst.rename(dst.path().with_file_name(filename))?;

    let dst_metadata = dst
        .metadata()
        .with_context(|| format!("failed to stat: {}", dst.path()))?;
    let compressed_size = dst_metadata.len();

    ws.config().ui().print(Status::new(
        "Packaged",
        &format!(
            "{} files, {:.1} ({:.1} compressed)",
            HumanCount(num_files as u64),
            HumanBytes(uncompressed_size),
            HumanBytes(compressed_size),
        ),
    ));

    Ok(dst)
}

fn list_one_impl(
    pkg_id: PackageId,
    opts: &PackageOpts,
    ws: &Workspace<'_>,
) -> Result<Vec<Utf8PathBuf>> {
    let pkg = ws.fetch_package(&pkg_id)?;
    let recipe = prepare_archive_recipe(pkg, opts)?;
    Ok(recipe.into_iter().map(|f| f.path).collect())
}

fn prepare_archive_recipe(pkg: &Package, opts: &PackageOpts) -> Result<ArchiveRecipe> {
    let mut recipe = source_files(pkg)?;

    // Sort the recipe before any checks, to ensure generated errors are reproducible.
    sort_recipe(&mut recipe);

    check_filenames(&recipe)?;
    check_no_reserved_files(&recipe)?;

    // Add normalized manifest file.
    recipe.push(ArchiveFile {
        path: MANIFEST_FILE_NAME.into(),
        contents: ArchiveFileContents::Generated({
            let pkg = pkg.clone();
            Box::new(|| normalize_manifest(pkg))
        }),
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

    // Add VCS info file.
    if let Ok(repo) = PackageRepository::open(pkg) {
        recipe.push(ArchiveFile {
            path: VCS_INFO_FILE_NAME.into(),
            contents: ArchiveFileContents::Generated({
                let opts = opts.clone();
                Box::new(move || {
                    let vcs_info = extract_vcs_info(repo, &opts)?;
                    Ok(serde_json::to_string(&vcs_info)?.into_bytes())
                })
            }),
        });
    };

    // Put generated files in right order within the recipe.
    sort_recipe(&mut recipe);

    // Assert there are no duplicates. We make use of the fact, that recipe is now sorted.
    assert!(
        recipe.windows(2).all(|w| w[0].path != w[1].path),
        "duplicate files in package recipe: {duplicates}",
        duplicates = recipe
            .windows(2)
            .filter(|w| w[0].path == w[1].path)
            .map(|w| w[0].path.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(recipe)
}

#[tracing::instrument(level = "trace", skip_all)]
fn source_files(pkg: &Package) -> Result<ArchiveRecipe> {
    list_source_files(pkg)?
        .into_iter()
        .map(|on_disk| {
            let path = on_disk.strip_prefix(pkg.root())?.to_owned();
            Ok(ArchiveFile {
                path,
                contents: ArchiveFileContents::OnDisk(on_disk),
            })
        })
        .collect()
}

fn check_no_reserved_files(recipe: &ArchiveRecipe) -> Result<()> {
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

fn check_filenames(recipe: &ArchiveRecipe) -> Result<()> {
    for ArchiveFile { path, .. } in recipe {
        const BAD_CHARS: &[char] = &['/', '\\', '<', '>', ':', '"', '|', '?', '*'];
        for component in path.components() {
            let name = component.as_str();
            if let Some(c) = BAD_CHARS.iter().find(|&&c| name.contains(c)) {
                bail!("cannot package a filename with a special character `{c}`: {path}");
            }
        }

        if restricted_names::is_windows_restricted_path(path.as_std_path()) {
            bail!("cannot package file `{path}`, it is a Windows reserved filename");
        }
    }
    Ok(())
}

/// Sort archive files alphabetically, putting the version file first.
fn sort_recipe(recipe: &mut ArchiveRecipe) {
    recipe.sort_unstable_by_key(|f| {
        let priority = if f.path == VERSION_FILE_NAME { 0 } else { 1 };
        (priority, f.path.clone())
    });
}

#[tracing::instrument(level = "trace", skip_all)]
fn normalize_manifest(pkg: Package) -> Result<Vec<u8>> {
    let mut buf = Vec::new();

    writedoc!(
        &mut buf,
        r##"
        # Code generated by scarb package -p {package_name}; DO NOT EDIT.
        #
        # When uploading packages to the registry Scarb will automatically
        # "normalize" {toml} files for maximal compatibility
        # with all versions of Scarb and also rewrite `path` dependencies
        # to registry dependencies.
        #
        # If you are reading this file be aware that the original {toml}
        # will likely look very different (and much more reasonable).
        # See {orig} for the original contents.
        "##,
        package_name = pkg.id.name,
        toml = MANIFEST_FILE_NAME,
        orig = ORIGINAL_MANIFEST_FILE_NAME,
    )?;
    writeln!(&mut buf)?;

    let manifest = prepare_manifest_for_publish(&pkg)?;
    let toml = toml::to_string_pretty(&manifest)?;
    writeln!(&mut buf, "{toml}")?;

    Ok(buf)
}

/// Compress and package the recipe, and write it into the given file.
///
/// Returns the uncompressed size of the contents of the archive.
#[tracing::instrument(level = "trace", skip_all)]
fn tar(
    pkg_id: PackageId,
    recipe: ArchiveRecipe,
    dst: &mut File,
    ws: &Workspace<'_>,
) -> Result<u64> {
    const COMPRESSION_LEVEL: i32 = 22;
    let encoder = zstd::stream::Encoder::new(dst, COMPRESSION_LEVEL)?;
    let mut ar = tar::Builder::new(encoder);

    let base_path = Utf8PathBuf::from(pkg_id.tarball_basename());

    let mut uncompressed_size = 0;
    for ArchiveFile { path, contents } in recipe {
        ws.config()
            .ui()
            .verbose(Status::new("Archiving", path.as_str()));

        let archive_path = base_path.join(&path);
        let mut header = tar::Header::new_gnu();
        match contents {
            ArchiveFileContents::OnDisk(disk_path) => {
                let mut file = File::open(&disk_path)
                    .with_context(|| format!("failed to open for archiving: {disk_path}"))?;

                let metadata = file
                    .metadata()
                    .with_context(|| format!("failed to stat: {disk_path}"))?;

                header.set_metadata_in_mode(&metadata, tar::HeaderMode::Deterministic);
                header.set_cksum();

                ar.append_data(&mut header, &archive_path, &mut file)
                    .with_context(|| format!("could not archive source file: {disk_path}"))?;

                uncompressed_size += metadata.len();
            }

            ArchiveFileContents::Generated(generator) => {
                let contents = generator()?;

                header.set_entry_type(tar::EntryType::file());
                header.set_mode(0o644);
                header.set_size(contents.len() as u64);

                // From `set_metadata_in_mode` implementation in `tar` crate:
                // We could in theory set the mtime to zero here, but not all
                // tools seem to behave well when ingesting files with a 0
                // timestamp.
                header.set_mtime(1);

                header.set_cksum();

                ar.append_data(&mut header, &archive_path, contents.as_slice())
                    .with_context(|| format!("could not archive source file: {path}"))?;

                uncompressed_size += contents.len() as u64;
            }
        }
    }

    let encoder = ar.into_inner()?;
    encoder.finish()?;
    Ok(uncompressed_size)
}
