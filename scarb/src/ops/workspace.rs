use anyhow::{Context, Result, anyhow};
use camino::{Utf8Path, Utf8PathBuf};
use glob::glob;
use indoc::formatdoc;
use indoc::indoc;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::PathBuf;
use tracing::trace;

use crate::MANIFEST_FILE_NAME;
use crate::core::TomlManifest;
use crate::core::config::Config;
use crate::core::package::Package;
use crate::core::source::SourceId;
use crate::core::workspace::Workspace;
use crate::ops::find_workspace_manifest_path;
use scarb_fs_utils as fsx;
use scarb_fs_utils::{PathBufUtf8Ext, is_hidden};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WorkspaceMemberDiscovery {
    pub members_manifests: Vec<Utf8PathBuf>,
    pub warnings: Vec<WorkspaceMemberWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceMemberWarning {
    MissingManifest { manifest_path: Utf8PathBuf },
}

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
    package_manifest: &Utf8Path,
    source_id: SourceId,
    config: &'c Config,
) -> Result<Workspace<'c>> {
    // Find workspace candidate, if any.
    // This is only a candidate, because it is not guaranteed to add the package as a member.
    let workspace_manifest =
        find_workspace_manifest_path(package_manifest.into())?.unwrap_or(package_manifest.into());
    let workspace_candidate = read_workspace_root(&workspace_manifest, source_id, config)?;
    // Check if the package is a member of the workspace candidate.
    if workspace_manifest == package_manifest
        || workspace_candidate
            .members()
            .any(|p| p.manifest_path() == package_manifest)
    {
        // If so, we found the package workspace.
        Ok(workspace_candidate)
    } else {
        // Otherwise, we need to create a virtual workspace
        read_workspace_root(package_manifest, source_id, config)
    }
}

/// Validates workspace root (virtual) manifest
pub fn validate_root_manifest(manifest: &TomlManifest) -> Result<()> {
    if manifest.is_package() {
        return Ok(());
    }

    if manifest.dependencies.is_some() {
        return Err(anyhow!(indoc! {r#"
            this virtual manifest specifies a [dependencies] section, which is not allowed
            help: use [workspace.dependencies] instead
        "#}));
    }

    if manifest.is_workspace() {
        return Ok(());
    }

    Err(anyhow!("the [package] section is missing"))
}

fn read_workspace_root<'c>(
    manifest_path: &Utf8Path,
    source_id: SourceId,
    config: &'c Config,
) -> Result<Workspace<'c>> {
    let toml_manifest = TomlManifest::read_from_path(manifest_path)?;
    let toml_workspace = toml_manifest.get_workspace();
    let profiles = toml_manifest.collect_profiles()?;

    validate_root_manifest(&toml_manifest)
        .with_context(|| format!("failed to parse manifest at: {manifest_path}"))?;

    let root_package = if toml_manifest.is_package() {
        let manifest = toml_manifest
            .to_manifest(
                manifest_path,
                manifest_path,
                source_id,
                config.profile(),
                &toml_manifest,
                config,
            )
            .with_context(|| format!("failed to parse manifest at: {manifest_path}"))?;
        let manifest = Box::new(manifest);
        let package = Package::new(manifest.summary.package_id, manifest_path.into(), manifest);
        Some(package)
    } else {
        None
    };

    let patch = toml_manifest.collect_patch(manifest_path)?;

    if let Some(workspace) = toml_workspace {
        let workspace_root = manifest_path
            .parent()
            .expect("Manifest path must have parent.");

        let scripts = workspace.scripts.unwrap_or_default();
        let member_discovery = workspace
            .members
            .map(|members| discover_workspace_member_manifests(workspace_root, &members))
            .transpose()?
            .unwrap_or_default();

        for warning in &member_discovery.warnings {
            match warning {
                WorkspaceMemberWarning::MissingManifest { manifest_path } => {
                    config.ui().warn(format!(
                        "workspace members definition matched path `{}`, \
                        which misses a manifest file",
                        manifest_path
                    ));
                }
            }
        }

        // Read workspace members.
        let mut packages = member_discovery
            .members_manifests
            .iter()
            .map(AsRef::as_ref)
            .map(|package_path| {
                let package_manifest = TomlManifest::read_from_path(package_path)?;
                // Read the member package.
                let manifest = package_manifest
                    .to_manifest(
                        package_path,
                        manifest_path,
                        source_id,
                        config.profile(),
                        &toml_manifest,
                        config,
                    )
                    .with_context(|| format!("failed to parse manifest at: {manifest_path}"))?;
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
        let require_audits = workspace.require_audits.unwrap_or(false);
        let allow_no_audits = workspace.allow_no_audits.unwrap_or_default();

        Workspace::new(
            manifest_path.into(),
            packages.as_ref(),
            root_package,
            config,
            profiles,
            scripts,
            patch,
            require_audits,
            allow_no_audits,
        )
    } else {
        // Read single package workspace
        let package = root_package.ok_or_else(|| anyhow!("the [package] section is missing"))?;
        Workspace::from_single_package(package, config, profiles, patch)
    }
}

pub fn discover_workspace_member_manifests(
    root: &Utf8Path,
    globs: &Vec<String>,
) -> Result<WorkspaceMemberDiscovery> {
    let mut members_manifests = Vec::with_capacity(globs.len());
    let mut warnings = Vec::new();

    for pattern in globs {
        for path in glob(root.join(&pattern).as_str())
            .with_context(|| format!("could not parse pattern: {pattern}"))?
        {
            let path =
                path.with_context(|| format!("unable to match path to pattern: {pattern}"))?;
            maybe_collect_member_manifest_path(path, &mut members_manifests, &mut warnings)?;
        }
    }

    Ok(WorkspaceMemberDiscovery {
        members_manifests,
        warnings,
    })
}

fn maybe_collect_member_manifest_path(
    path: PathBuf,
    manifests: &mut Vec<Utf8PathBuf>,
    warnings: &mut Vec<WorkspaceMemberWarning>,
) -> Result<()> {
    if is_hidden(path.clone()) {
        return Ok(());
    }

    let manifest_path = path.join(MANIFEST_FILE_NAME);
    if manifest_path.is_file() {
        manifests.push(fsx::canonicalize_utf8(manifest_path)?);
    } else {
        warnings.push(WorkspaceMemberWarning::MissingManifest {
            manifest_path: manifest_path.try_into_utf8()?,
        });
    }

    Ok(())
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
            let is_hidden = is_hidden(entry.path());
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

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::core::TomlManifest;
    use indoc::indoc;
    use tempfile::Builder;

    use super::{
        WorkspaceMemberWarning, discover_workspace_member_manifests, validate_root_manifest,
    };

    #[test]
    fn discovers_workspace_member_manifests_from_direct_and_glob_patterns() {
        let temp = Builder::new().prefix("scarb-workspace-").tempdir().unwrap();
        fs::create_dir_all(temp.path().join("member-a")).unwrap();
        fs::create_dir_all(temp.path().join("members/member-b")).unwrap();
        fs::create_dir_all(temp.path().join(".hidden")).unwrap();
        fs::write(temp.path().join("member-a/Scarb.toml"), "").unwrap();
        fs::write(temp.path().join("members/member-b/Scarb.toml"), "").unwrap();
        fs::write(temp.path().join(".hidden/Scarb.toml"), "").unwrap();

        let root = camino::Utf8Path::from_path(temp.path()).unwrap();
        let members = vec![
            "member-a".to_string(),
            "members/*".to_string(),
            ".hidden".to_string(),
        ];

        let discovery = discover_workspace_member_manifests(root, &members).unwrap();

        assert_eq!(discovery.members_manifests.len(), 2);
        assert!(
            discovery
                .members_manifests
                .iter()
                .any(|manifest| manifest.ends_with("member-a/Scarb.toml"))
        );
        assert!(
            discovery
                .members_manifests
                .iter()
                .any(|manifest| manifest.ends_with("members/member-b/Scarb.toml"))
        );
    }

    #[test]
    fn reports_warning_for_member_match_without_manifest() {
        let temp = Builder::new().prefix("scarb-workspace-").tempdir().unwrap();
        fs::create_dir_all(temp.path().join("member-a/src")).unwrap();
        fs::write(temp.path().join("member-a/src/lib.cairo"), "fn main() {}\n").unwrap();

        let root = camino::Utf8Path::from_path(temp.path()).unwrap();
        let members = vec!["member-a".to_string()];

        let discovery = discover_workspace_member_manifests(root, &members).unwrap();

        assert!(discovery.members_manifests.is_empty());
        assert_eq!(
            discovery.warnings,
            vec![WorkspaceMemberWarning::MissingManifest {
                manifest_path: root.join("member-a/Scarb.toml")
            }]
        );
    }

    #[test]
    fn validates_virtual_workspace_root_manifest_rules() {
        let manifest = TomlManifest::read_from_str(indoc! {r#"
            [workspace]
            members = []

            [dependencies]
            foo = "1.0.0"
        "#})
        .unwrap();

        let error = validate_root_manifest(&manifest).unwrap_err();

        assert!(format!("{error:#}").contains(
            "this virtual manifest specifies a [dependencies] section, which is not allowed"
        ));
    }

    #[test]
    fn rejects_root_manifest_without_package_or_workspace() {
        let manifest = TomlManifest::read_from_str("").unwrap();

        let error = validate_root_manifest(&manifest).unwrap_err();

        assert!(format!("{error:#}").contains("the [package] section is missing"));
    }
}
