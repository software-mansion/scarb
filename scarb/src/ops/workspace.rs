use std::collections::hash_map::Entry;
use std::collections::HashMap;

use anyhow::{Context, Result};
use camino::Utf8Path;
use indoc::formatdoc;
use tracing::trace;

use crate::core::config::Config;
use crate::core::package::Package;
use crate::core::source::SourceId;
use crate::core::workspace::Workspace;
use crate::internal::fsx::PathBufUtf8Ext;
use crate::{ops, MANIFEST_FILE_NAME};

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
    let manifest = Box::new(ops::read_manifest(
        manifest_path,
        source_id,
        config.profile(),
    )?);

    let package = Package::new(manifest.summary.package_id, manifest_path.into(), manifest);

    Workspace::from_single_package(package, config)
}

#[tracing::instrument(level = "debug", skip(config))]
pub fn find_workspaces_recursive_with_source_id<'c>(
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
            let is_hidden = entry
                .file_name()
                .to_str()
                .map(|s| s.starts_with('.'))
                .unwrap_or(false);
            if is_hidden {
                return false;
            }

            // Do not walk into workspaces subdirectories.
            let is_in_workspace = entry
                .path()
                .parent()
                .map(|p| p.join(MANIFEST_FILE_NAME).exists())
                .unwrap_or(false);
            if is_in_workspace {
                return false;
            }

            true
        } else {
            false
        }
    }

    fn inner<'c>(
        root: &Utf8Path,
        source_id: SourceId,
        config: &'c Config,
    ) -> Result<Vec<Workspace<'c>>> {
        let mut found = Vec::new();

        let walker = WalkDir::new(root).into_iter().filter_entry(filter_entry);
        for entry in walker {
            let path = entry.context("failed to traverse directory")?.into_path();
            let manifest_path = path.join(MANIFEST_FILE_NAME);
            trace!(manifest_path=%manifest_path.display());
            if manifest_path.exists() {
                let manifest_path = manifest_path.try_into_utf8()?;
                let ws = read_workspace_with_source_id(&manifest_path, source_id, config)?;
                found.push(ws);
            }
        }

        Ok(found)
    }

    inner(root, source_id, config).with_context(|| format!("failed to find workspaces in: {root}"))
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

    let workspaces = find_workspaces_recursive_with_source_id(root, source_id, config)?;

    let mut found = HashMap::new();

    for ws in workspaces {
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
