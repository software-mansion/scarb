use std::ops::Range;

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use scarb_ui::Verbosity;
use serde::{Deserialize, Serialize};
use smol_str::ToSmolStr;
use toml::de::Error as TomlParseError;

use crate::compiler::Profile;
use crate::core::{Config, SourceId, TomlManifest};
use crate::ops::{
    discover_workspace_member_manifests, find_workspace_manifest_path, read_workspace,
    validate_root_manifest,
};

type ManifestOffset = u32;

/// Byte-offset span in a manifest file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestRange {
    pub start: ManifestOffset,
    pub end: ManifestOffset,
}

/// A manifest validation diagnostic with file path and optional span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestDiagnostic {
    pub file: Utf8PathBuf,
    pub message: String,
    pub span: Option<ManifestRange>,
}

impl ManifestDiagnostic {
    /// Builds a manifest diagnostic from an error at a specific manifest path.
    pub fn from_error(error: anyhow::Error, manifest_path: &Utf8Path) -> Self {
        let span = error
            .chain()
            .find_map(|cause| {
                cause
                    .downcast_ref::<TomlParseError>()
                    .and_then(TomlParseError::span)
            })
            .map(ManifestRange::from);

        ManifestDiagnostic {
            file: manifest_path.to_path_buf(),
            message: format!("{error:#}"),
            span,
        }
    }
}

/// Validation output that may contain zero or more diagnostics.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ManifestValidationResult {
    pub diagnostics: Vec<ManifestDiagnostic>,
}

impl ManifestValidationResult {
    /// Builds a validation result containing one diagnostic derived from an error.
    pub fn from_single_error(error: anyhow::Error, manifest_path: &Utf8Path) -> Self {
        ManifestValidationResult {
            diagnostics: vec![ManifestDiagnostic::from_error(error, manifest_path)],
        }
    }

    /// Returns `true` when validation produced no diagnostics.
    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Input for manifest validation with optional profile context.
#[derive(Debug, Clone)]
pub struct ManifestValidationInput<'a> {
    pub manifest_path: &'a Utf8Path,
    pub profile: Option<&'a str>,
}

impl<'a> ManifestValidationInput<'a> {
    /// Creates validation input for a manifest path.
    pub fn new(manifest_path: &'a Utf8Path) -> Self {
        Self {
            manifest_path,
            profile: None,
        }
    }

    pub fn new_for_profile(manifest_path: &'a Utf8Path, profile: &'a str) -> Self {
        Self {
            manifest_path,
            profile: Some(profile),
        }
    }
}

/// Finds the nearest workspace root manifest for the provided manifest path.
pub fn discover_workspace_manifest_path(manifest_path: &Utf8Path) -> Result<Option<Utf8PathBuf>> {
    find_workspace_manifest_path(manifest_path.to_path_buf())
}

fn collect_declared_profiles(manifest_path: &Utf8Path) -> Result<Vec<String>> {
    let manifest_contents = read_manifest_contents(manifest_path)?;
    let manifest = parse_manifest(manifest_path, &manifest_contents)?;
    let profiles = manifest
        .collect_profiles()?
        .into_iter()
        .map(|profile| profile.as_str().to_string())
        .collect::<Vec<_>>();
    Ok(profiles)
}

pub fn validate_manifest(manifest_path: &Utf8Path) -> ManifestValidationResult {
    let base_result = validate_manifest_with_profile(ManifestValidationInput::new(manifest_path));
    if !base_result.is_valid() {
        return base_result;
    }

    let mut diagnostics = base_result.diagnostics;
    let declared_profiles = collect_declared_profiles(manifest_path)
        .ok()
        .unwrap_or_default();
    for profile in declared_profiles {
        let input = ManifestValidationInput::new_for_profile(manifest_path, profile.as_str());
        diagnostics.extend(validate_manifest_with_profile(input).diagnostics);
    }

    ManifestValidationResult { diagnostics }
}

fn validate_manifest_with_profile(input: ManifestValidationInput<'_>) -> ManifestValidationResult {
    let profile = match resolve_profile(input.profile) {
        Ok(profile) => profile,
        Err(error) => {
            return ManifestValidationResult::from_single_error(error, input.manifest_path);
        }
    };

    let config = match build_config(input.manifest_path, profile) {
        Ok(config) => config,
        Err(error) => {
            return ManifestValidationResult::from_single_error(error, input.manifest_path);
        }
    };

    let manifest_contents = match read_manifest_contents(input.manifest_path) {
        Ok(contents) => contents,
        Err(error) => {
            return ManifestValidationResult::from_single_error(error, input.manifest_path);
        }
    };

    let manifest = match parse_manifest(input.manifest_path, &manifest_contents) {
        Ok(manifest) => manifest,
        Err(error) => {
            return ManifestValidationResult::from_single_error(error, input.manifest_path);
        }
    };

    if let Err(error) = validate_common_manifest_sections(input.manifest_path, &manifest) {
        return ManifestValidationResult::from_single_error(error, input.manifest_path);
    }

    if let Err(error) = validate_root_manifest(&manifest)
        .with_context(|| format!("failed to parse manifest at: {}", input.manifest_path))
    {
        return ManifestValidationResult::from_single_error(error, input.manifest_path);
    }

    if manifest.is_package() {
        let source_id = match SourceId::for_path(input.manifest_path)
            .with_context(|| format!("failed to parse manifest at: {}", input.manifest_path))
        {
            Ok(source_id) => source_id,
            Err(error) => {
                return ManifestValidationResult::from_single_error(error, input.manifest_path);
            }
        };

        if let Err(error) = manifest
            .to_manifest(
                input.manifest_path,
                input.manifest_path,
                source_id,
                config.profile(),
                &manifest,
                &config,
            )
            .with_context(|| format!("failed to parse manifest at: {}", input.manifest_path))
        {
            return ManifestValidationResult::from_single_error(error, input.manifest_path);
        }
    }

    ManifestValidationResult::default()
}

pub fn validate_workspace(manifest_path: &Utf8Path) -> ManifestValidationResult {
    let workspace_root_manifest_path = discover_workspace_manifest_path(manifest_path)
        .ok()
        .flatten();
    let workspace_root_manifest_path = workspace_root_manifest_path
        .as_deref()
        .unwrap_or(manifest_path);

    let base_result =
        validate_workspace_with_profile(ManifestValidationInput::new(workspace_root_manifest_path));
    if !base_result.is_valid() {
        return base_result;
    }

    let mut diagnostics = vec![];
    let declared_profiles = collect_declared_profiles(workspace_root_manifest_path)
        .ok()
        .unwrap_or_default();
    for profile in declared_profiles {
        diagnostics.extend(
            validate_workspace_with_profile(ManifestValidationInput::new_for_profile(
                workspace_root_manifest_path,
                profile.as_str(),
            ))
            .diagnostics,
        );
    }

    ManifestValidationResult { diagnostics }
}

pub fn validate_workspace_with_profile(
    input: ManifestValidationInput<'_>,
) -> ManifestValidationResult {
    let workspace_root_manifest_path = input.manifest_path;
    let profile = match resolve_profile(input.profile) {
        Ok(profile) => profile,
        Err(error) => {
            return ManifestValidationResult::from_single_error(
                error,
                workspace_root_manifest_path,
            );
        }
    };

    let config = match build_config(workspace_root_manifest_path, profile) {
        Ok(config) => config,
        Err(error) => {
            return ManifestValidationResult::from_single_error(
                error,
                workspace_root_manifest_path,
            );
        }
    };

    let root_contents = match read_manifest_contents(workspace_root_manifest_path) {
        Ok(contents) => contents,
        Err(error) => {
            return ManifestValidationResult::from_single_error(
                error,
                workspace_root_manifest_path,
            );
        }
    };

    let root_manifest = match parse_manifest(workspace_root_manifest_path, &root_contents) {
        Ok(manifest) => manifest,
        Err(error) => {
            return ManifestValidationResult::from_single_error(
                error,
                workspace_root_manifest_path,
            );
        }
    };

    let mut diagnostics = Vec::new();

    if let Err(error) =
        validate_common_manifest_sections(workspace_root_manifest_path, &root_manifest)
    {
        diagnostics.push(ManifestDiagnostic::from_error(
            error,
            workspace_root_manifest_path,
        ));
    }

    if let Err(error) = validate_root_manifest(&root_manifest)
        .with_context(|| format!("failed to parse manifest at: {workspace_root_manifest_path}"))
    {
        diagnostics.push(ManifestDiagnostic::from_error(
            error,
            workspace_root_manifest_path,
        ));
    }

    if diagnostics.is_empty() && root_manifest.is_package() {
        let source_id = match SourceId::for_path(workspace_root_manifest_path)
            .with_context(|| format!("failed to parse manifest at: {workspace_root_manifest_path}"))
        {
            Ok(source_id) => source_id,
            Err(error) => {
                diagnostics.push(ManifestDiagnostic::from_error(
                    error,
                    workspace_root_manifest_path,
                ));
                return ManifestValidationResult { diagnostics };
            }
        };

        if let Err(error) = root_manifest
            .to_manifest(
                workspace_root_manifest_path,
                workspace_root_manifest_path,
                source_id,
                config.profile(),
                &root_manifest,
                &config,
            )
            .with_context(|| format!("failed to parse manifest at: {workspace_root_manifest_path}"))
        {
            diagnostics.push(ManifestDiagnostic::from_error(
                error,
                workspace_root_manifest_path,
            ));
        }
    }

    if !diagnostics.is_empty() {
        return ManifestValidationResult { diagnostics };
    }

    let member_discovery = match root_manifest
        .get_workspace()
        .and_then(|workspace| workspace.members)
    {
        Some(members) => {
            let workspace_root = workspace_root_manifest_path
                .parent()
                .expect("manifest path parent must always exist");
            match discover_workspace_member_manifests(workspace_root, &members) {
                Ok(discovery) => discovery,
                Err(error) => {
                    diagnostics.push(ManifestDiagnostic::from_error(
                        error,
                        workspace_root_manifest_path,
                    ));
                    return ManifestValidationResult { diagnostics };
                }
            }
        }
        None => Default::default(),
    };

    for member_manifest_path in member_discovery.members_manifests {
        if let Some(diagnostic) = validate_workspace_member(
            &member_manifest_path,
            workspace_root_manifest_path,
            &root_manifest,
            &config,
        ) {
            diagnostics.push(diagnostic);
        }
    }

    if !diagnostics.is_empty() {
        return ManifestValidationResult { diagnostics };
    }

    if let Err(error) = read_workspace(workspace_root_manifest_path, &config) {
        diagnostics.push(ManifestDiagnostic::from_error(
            error,
            workspace_root_manifest_path,
        ));
    }

    ManifestValidationResult { diagnostics }
}

fn read_manifest_contents(manifest_path: &Utf8Path) -> Result<String> {
    std::fs::read_to_string(manifest_path)
        .with_context(|| format!("failed to read manifest at: {manifest_path}"))
}

fn parse_manifest(manifest_path: &Utf8Path, manifest_contents: &str) -> Result<TomlManifest> {
    TomlManifest::read_from_str(manifest_contents)
        .with_context(|| format!("failed to parse manifest at: {manifest_path}"))
}

fn validate_common_manifest_sections(
    manifest_path: &Utf8Path,
    manifest: &TomlManifest,
) -> Result<()> {
    manifest
        .collect_profiles()
        .with_context(|| format!("failed to parse manifest at: {manifest_path}"))?;

    manifest
        .collect_patch(manifest_path)
        .with_context(|| format!("failed to parse manifest at: {manifest_path}"))?;

    Ok(())
}

fn validate_workspace_member(
    member_manifest_path: &Utf8Path,
    workspace_root_manifest_path: &Utf8Path,
    root_manifest: &TomlManifest,
    config: &Config,
) -> Option<ManifestDiagnostic> {
    let member_contents = match read_manifest_contents(member_manifest_path) {
        Ok(contents) => contents,
        Err(error) => {
            return Some(ManifestDiagnostic::from_error(error, member_manifest_path));
        }
    };

    let member_manifest = match parse_manifest(member_manifest_path, &member_contents) {
        Ok(manifest) => manifest,
        Err(error) => {
            return Some(ManifestDiagnostic::from_error(error, member_manifest_path));
        }
    };

    if let Err(error) = validate_common_manifest_sections(member_manifest_path, &member_manifest) {
        return Some(ManifestDiagnostic::from_error(error, member_manifest_path));
    }

    let source_id = match SourceId::for_path(member_manifest_path)
        .with_context(|| format!("failed to parse manifest at: {member_manifest_path}"))
    {
        Ok(source_id) => source_id,
        Err(error) => {
            return Some(ManifestDiagnostic::from_error(error, member_manifest_path));
        }
    };

    if let Err(error) = member_manifest
        .to_manifest(
            member_manifest_path,
            workspace_root_manifest_path,
            source_id,
            config.profile(),
            root_manifest,
            config,
        )
        .with_context(|| format!("failed to parse manifest at: {member_manifest_path}"))
    {
        return Some(ManifestDiagnostic::from_error(error, member_manifest_path));
    }

    None
}

fn build_config(manifest_path: &Utf8Path, profile: Profile) -> Result<Config> {
    Config::builder(manifest_path.to_path_buf())
        .profile(profile)
        .ui_verbosity(Verbosity::Quiet)
        .build()
}

fn resolve_profile(profile: Option<&str>) -> Result<Profile> {
    if let Some(profile_name) = profile {
        return Profile::try_new(profile_name.to_smolstr())
            .with_context(|| format!("invalid profile `{profile_name}`"));
    }
    Ok(Profile::default())
}

impl From<Range<usize>> for ManifestRange {
    fn from(value: Range<usize>) -> Self {
        ManifestRange {
            start: value.start as ManifestOffset,
            end: value.end as ManifestOffset,
        }
    }
}
