use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use glob::glob;
use indoc::formatdoc;
use tracing::trace;

use crate::core::config::Config;
use crate::core::package::Package;
use crate::core::source::SourceId;
use crate::core::workspace::Workspace;
use crate::core::TomlManifest;
use crate::internal::fsx::PathBufUtf8Ext;
use crate::process::is_hidden;
use crate::MANIFEST_FILE_NAME;

#[tracing::instrument(level = "debug", skip(config))]
pub fn read_workspace<'c>(manifest_path: &Utf8Path, config: &'c Config) -> Result<Workspace<'c>> {
    let source_id = SourceId::for_path(manifest_path)?;
    read_workspace_impl(manifest_path, source_id, config)
}

#[tracing::instrument(level = "debug", skip(config))]
pub fn read_workspace_with_source_id<'c>(
    manifest_path: &Utf8Path,
    source_id: SourceId,
    config: &'c Config,
) -> Result<Workspace<'c>> {
    read_workspace_impl(manifest_path, source_id, config)
}

fn read_workspace_impl<'c>(
    manifest_path: &Utf8Path,
    source_id: SourceId,
    config: &'c Config,
) -> Result<Workspace<'c>> {
    let toml_manifest = TomlManifest::read_from_path(manifest_path)?;
    let toml_workspace = toml_manifest.get_workspace();

    let root_package = if toml_manifest.is_package() {
        let manifest = toml_manifest
            .to_manifest(
                manifest_path,
                source_id,
                config.profile(),
                toml_workspace.clone(),
            )
            .with_context(|| format!("failed to parse manifest at `{manifest_path}`"))?;
        let manifest = Box::new(manifest);
        let package = Package::new(manifest.summary.package_id, manifest_path.into(), manifest);
        Some(package)
    } else {
        None
    };

    if let Some(workspace) = toml_workspace {
        let workspace_root = manifest_path
            .parent()
            .expect("Manifest path must have parent.");

        // Read workspace members.
        let mut packages = workspace
            .members
            .map(|m| find_member_paths(workspace_root, m))
            .unwrap_or_else(|| Ok(Vec::new()))?
            .iter()
            .map(AsRef::as_ref)
            .map(|package_path| {
                let package_manifest = TomlManifest::read_from_path(package_path)?;
                // Read the member package.
                let manifest = package_manifest
                    .to_manifest(
                        package_path,
                        source_id,
                        config.profile(),
                        Some(workspace.clone()),
                    )
                    .with_context(|| format!("failed to parse manifest at `{manifest_path}`"))?;
                let manifest = Box::new(manifest);
                let package =
                    Package::new(manifest.summary.package_id, package_path.into(), manifest);
                Ok(package)
            })
            .collect::<Result<Vec<_>>>()?;
        // Read root package.
        let root_package = root_package.map(|p| {
            packages.push(p.clone());
            p.id
        });
        Workspace::new(
            manifest_path.into(),
            packages.as_ref(),
            root_package,
            config,
        )
    } else {
        // Read single package workspace
        let package = root_package.ok_or_else(|| anyhow!("the [package] section is missing"))?;
        Workspace::from_single_package(package, config)
    }
}

fn find_member_paths(root: &Utf8Path, globs: Vec<String>) -> Result<Vec<Utf8PathBuf>> {
    globs
        .iter()
        .map(|path| {
            // Expand globs from workspace root.
            glob(root.join(path).to_string().as_str())
                .with_context(|| format!("could not parse pattern `{}`", &path))?
                .map(|p| p.with_context(|| format!("unable to match path to pattern `{}`", &path)))
                .map(|p| {
                    // Return manifest path.
                    p.map(|p| p.join(MANIFEST_FILE_NAME))
                        .map(PathBuf::try_into_utf8)
                })
                .collect::<Result<Result<Vec<_>, _>>>()?
        })
        .collect::<Result<Vec<_>>>()
        .map(|v| {
            v.into_iter()
                .flatten()
                // Make sure all files exist.
                .filter(|p| p.is_file())
                .collect::<Vec<_>>()
        })
}

#[tracing::instrument(level = "debug", skip(config))]
pub fn find_all_workspaces_recursive_with_source_id<'c>(
    root: &Utf8Path,
    source_id: SourceId,
    config: &'c Config,
) -> Result<Vec<Workspace<'c>>> {
    use walkdir::{DirEntry, WalkDir};

    fn filter_entry(entry: &DirEntry) -> bool {
        if entry.file_type().is_file() {
            // As for files, we are only interested in standard named manifest files.
            entry.file_name() == MANIFEST_FILE_NAME
        } else if entry.file_type().is_dir() {
            // Do not walk into hidden directories.
            let is_hidden = is_hidden(entry);
            // Do not traverse package directories.
            let is_package = entry
                .path()
                .parent()
                .map(|p| p.join(MANIFEST_FILE_NAME).exists())
                .unwrap_or(false);
            !is_hidden && !is_package
        } else {
            false
        }
    }

    let inner = |root: &Utf8Path| -> Result<Vec<Workspace<'c>>> {
        let mut found = Vec::new();
        let walker = WalkDir::new(root).into_iter().filter_entry(filter_entry);
        for entry in walker {
            let path = entry.context("failed to traverse directory")?.into_path();
            let manifest_path = path.join(MANIFEST_FILE_NAME);
            trace!(manifest_path=%manifest_path.display());
            if manifest_path.exists() {
                let manifest_path = manifest_path.try_into_utf8()?;
                let ws = read_workspace_root(&manifest_path, source_id, config)?;
                found.push(ws);
            }
        }
        Ok(found)
    };

    inner(root).with_context(|| format!("failed to find workspaces in: {root}"))
}

#[tracing::instrument(level = "debug", skip(config))]
pub fn find_all_packages_recursive_with_source_id(
    root: &Utf8Path,
    source_id: SourceId,
    config: &Config,
) -> Result<Vec<Package>> {
    fn relative_source_path(source_id: SourceId, base: SourceId) -> String {
        let Some(source_path) = source_id.to_path() else {
            return source_id.to_string();
        };

        let Some(base_path) = base.to_path() else {
            return source_id.to_string();
        };

        let Ok(relative_path) = source_path.strip_prefix(base_path) else {
            return source_path.to_string();
        };

        relative_path.to_string()
    }
    let mut found = HashMap::new();
    for ws in find_all_workspaces_recursive_with_source_id(root, source_id, config)? {
        for pkg in ws.members() {
            match found.entry(pkg.id) {
                Entry::Vacant(e) => {
                    e.insert(pkg);
                }
                Entry::Occupied(e) => {
                    config.ui().warn({
                        let path_a = relative_source_path(e.key().source_id, source_id);
                        let path_b = relative_source_path(pkg.id.source_id, source_id);
                        formatdoc! {"
                            found duplicate packages named `{pkg}`

                            Found locations:
                            - {path_a}
                            - {path_b}

                            Because of this, referencing package `{pkg}` will fail.
                        "}
                    });

                    e.remove();
                }
            }
        }
    }
    Ok(found.into_values().collect())
}
