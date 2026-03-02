use std::ops::Range;

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use scarb::compiler::Profile;
use scarb::core::{Config, SourceId, TomlManifest};
use scarb::ops::{
    discover_workspace_member_manifests, find_workspace_manifest_path, read_workspace,
    validate_root_manifest,
};
use scarb_ui::Verbosity;
use smol_str::ToSmolStr;
use toml::de::Error as TomlParseError;

type ManifestOffset = u32;

/// Byte-offset span in a manifest file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestRange {
    pub start: ManifestOffset,
    pub end: ManifestOffset,
}

/// A manifest validation diagnostic with file path and optional span.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Default, Clone)]
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

/// Collects profiles declared under `[profile.*]` in a manifest file.
fn collect_declared_profiles(manifest_path: &Utf8Path) -> Result<Vec<String>> {
    let manifest_contents = read_manifest_contents(manifest_path)?;
    let manifest = parse_manifest(manifest_path, &manifest_contents)?;
    let profiles = manifest
        // Scarb: validate and normalize `[profile.*]` declarations.
        .collect_profiles()?
        .into_iter()
        .map(|profile| profile.as_str().to_string())
        .collect::<Vec<_>>();
    Ok(profiles)
}

/// Validates one manifest, including declared-profile reruns after the base pass succeeds.
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

/// Validates one manifest using Scarb semantic/business rules.
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

    // Scarb: validate root-manifest shape for package/workspace roots,
    // including virtual-manifest restrictions such as forbidden `[dependencies]`.
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

        // Scarb: perform package semantic validation for this manifest in its own workspace context.
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

/// Validates a workspace, including declared-profile reruns after the base pass succeeds.
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

/// Validates a workspace root manifest and its member manifests.
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

    // Scarb: validate root-manifest shape before attempting member or workspace assembly.
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

        // Scarb: validate the root package semantically against the root workspace context.
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
            // Scarb: expand `workspace.members`, handling direct paths, globs, hidden paths and deduplication.
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

    // Scarb: assemble the full workspace and run cross-member/business-rule checks that require it.
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
        // Scarb: validate and normalize `[profile.*]` definitions used by later semantic checks.
        .collect_profiles()
        .with_context(|| format!("failed to parse manifest at: {manifest_path}"))?;

    manifest
        // Scarb: validate `[patch]` sections and dependency patch declarations.
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

    // Scarb: validate a workspace member package semantically against the root workspace manifest.
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

/// Config needed for later validations
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

#[cfg(test)]
mod tests {
    use std::fs;

    use indoc::indoc;
    use tempfile::tempdir;

    use super::{
        ManifestValidationInput, collect_declared_profiles, validate_manifest,
        validate_workspace_with_profile,
    };

    #[test]
    fn reports_toml_parse_error_with_span() {
        let dir = tempdir().unwrap();
        let manifest_path = dir.path().join("Scarb.toml");
        fs::write(&manifest_path, "[package]\nname = 1\n").unwrap();
        let manifest_path = camino::Utf8Path::from_path(&manifest_path).unwrap();

        let result = validate_manifest(manifest_path);

        assert!(!result.is_valid());
        assert!(result.diagnostics[0].span.is_some());
        assert_eq!(result.diagnostics[0].file, manifest_path);
    }

    #[test]
    fn reports_member_manifest_path_without_parsing_error_strings() {
        let dir = tempdir().unwrap();
        let workspace_manifest_path = dir.path().join("Scarb.toml");
        let member_dir = dir.path().join("member");
        fs::create_dir_all(member_dir.join("src")).unwrap();

        fs::write(
            &workspace_manifest_path,
            indoc! {r#"
                [workspace]
                members = ["member"]
            "#},
        )
        .unwrap();

        let member_manifest_path = member_dir.join("Scarb.toml");
        fs::write(
            &member_manifest_path,
            indoc! {r#"
                [package]
                name = 1
                version = "0.1.0"
            "#},
        )
        .unwrap();

        let workspace_manifest_path =
            camino::Utf8Path::from_path(&workspace_manifest_path).unwrap();
        let result =
            validate_workspace_with_profile(ManifestValidationInput::new(workspace_manifest_path));

        assert!(!result.is_valid());
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn collects_declared_profiles_from_manifest_contents() {
        let dir = tempdir().unwrap();
        let manifest_path = dir.path().join("Scarb.toml");
        fs::write(
            &manifest_path,
            indoc! {r#"
                [package]
                name = "foo"
                version = "0.1.0"
                edition = "2025_12"

                [profile.alpha]
                [profile.beta]
            "#},
        )
        .unwrap();
        let manifest_path = camino::Utf8Path::from_path(&manifest_path).unwrap();
        let profiles = collect_declared_profiles(manifest_path).unwrap();

        assert_eq!(profiles, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn validates_profile_specific_manifest_rules_for_declared_profiles() {
        let dir = tempdir().unwrap();
        let manifest_path = dir.path().join("Scarb.toml");
        fs::write(
            &manifest_path,
            indoc! {r#"
                [package]
                name = "manifest_diagnostics_ws"
                version = "0.1.0"
                edition = "2025_12"

                [profile.some-profile]

                [profile.custom]
                inherits = "some-profile"
            "#},
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.cairo"), "fn main() {}\n").unwrap();
        let manifest_path = camino::Utf8Path::from_path(&manifest_path).unwrap();

        let result = validate_manifest(manifest_path);

        assert!(!result.is_valid());
        assert!(result.diagnostics.iter().any(|diag| {
            diag.message
                .contains("profile can inherit from `dev` or `release` only")
        }));
    }
}
