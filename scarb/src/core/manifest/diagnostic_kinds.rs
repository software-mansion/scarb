use crate::core::{PackageName, TargetKind};
use camino::Utf8PathBuf;
use smol_str::SmolStr;
use thiserror::Error;
use toml_edit::Document;
use url::ParseError as UrlParseError;

use super::ManifestDiagnosticData;
use super::diagnostic::resolve_anchor_in_doc;
use super::{
    ManifestDependencyTable, ManifestDiagnosticAnchor, ManifestRelatedAnchor,
    ManifestRelatedLocation,
};

/// Typed manifest validation errors that carry semantic anchors for diagnostic span resolution.
///
/// This enum is the dispatch point for all manifest validation errors that can be anchored
/// to a specific location in the TOML source. New error types are added as variants here
/// in subsequent commits.
#[derive(Debug, Clone, Error)]
pub enum ManifestSemanticError {
    #[error(transparent)]
    ProfileNameInvalid(#[from] ProfileNameInvalid),
    #[error(transparent)]
    ProfileInheritanceInvalid(#[from] ProfileInheritanceInvalid),
    #[error(transparent)]
    ProfileCairoConflict(#[from] ProfileCairoConflict),
    #[error(transparent)]
    DependencyWorkspaceNotFound(#[from] DependencyWorkspaceNotFound),
    #[error(transparent)]
    DependencyGitRefWithoutGit(#[from] DependencyGitRefWithoutGit),
    #[error(transparent)]
    DependencyGitReferenceAmbiguous(#[from] DependencyGitReferenceAmbiguous),
    #[error(transparent)]
    DependencySourceMissing(#[from] DependencySourceMissing),
    #[error(transparent)]
    DependencyGitPathAmbiguous(#[from] DependencyGitPathAmbiguous),
    #[error(transparent)]
    DependencyGitRegistryAmbiguous(#[from] DependencyGitRegistryAmbiguous),
    #[error(transparent)]
    PatchNotInWorkspaceRoot(#[from] PatchNotInWorkspaceRoot),
    #[error(transparent)]
    PatchSourceConflict(#[from] PatchSourceConflict),
    #[error(transparent)]
    PatchSourceInvalidUrl(#[from] PatchSourceInvalidUrl),
    #[error(transparent)]
    ReadmePathInvalid(#[from] ReadmePathInvalid),
    #[error(transparent)]
    LicensePathInvalid(#[from] LicensePathInvalid),
    #[error(transparent)]
    DuplicateDefaultTargetDefinition(#[from] DuplicateDefaultTargetDefinition),
    #[error(transparent)]
    DuplicateNamedTargetDefinition(#[from] DuplicateNamedTargetDefinition),
}

impl ManifestSemanticError {
    /// Resolves this error's anchor(s) to byte spans using the raw manifest source.
    pub fn resolve(&self, manifest_source: &str) -> ManifestDiagnosticData {
        let Ok(document) = Document::parse(manifest_source) else {
            return ManifestDiagnosticData {
                span: None,
                related: vec![],
            };
        };
        let root = document.as_table();

        let span = self
            .primary_anchor()
            .and_then(|anchor| resolve_anchor_in_doc(root, &anchor));
        let related = self
            .related_anchors()
            .into_iter()
            .filter_map(|r| {
                resolve_anchor_in_doc(root, &r.anchor).map(|span| ManifestRelatedLocation {
                    message: r.message,
                    span,
                })
            })
            .collect();

        ManifestDiagnosticData { span, related }
    }

    fn primary_anchor(&self) -> Option<ManifestDiagnosticAnchor> {
        match self {
            Self::ProfileNameInvalid(e) => Some(e.primary_anchor()),
            Self::ProfileInheritanceInvalid(e) => Some(e.primary_anchor()),
            Self::ProfileCairoConflict(e) => Some(e.primary_anchor()),
            Self::DependencyWorkspaceNotFound(e) => Some(e.primary_anchor()),
            Self::DependencyGitRefWithoutGit(e) => Some(e.primary_anchor()),
            Self::DependencyGitReferenceAmbiguous(e) => Some(e.primary_anchor()),
            Self::DependencySourceMissing(e) => Some(e.primary_anchor()),
            Self::DependencyGitPathAmbiguous(e) => Some(e.primary_anchor()),
            Self::DependencyGitRegistryAmbiguous(e) => Some(e.primary_anchor()),
            Self::PatchNotInWorkspaceRoot(e) => Some(e.primary_anchor()),
            Self::PatchSourceConflict(e) => Some(e.primary_anchor()),
            Self::PatchSourceInvalidUrl(e) => Some(e.primary_anchor()),
            Self::ReadmePathInvalid(e) => e.primary_anchor(),
            Self::LicensePathInvalid(e) => e.primary_anchor(),
            Self::DuplicateDefaultTargetDefinition(e) => Some(e.primary_anchor()),
            Self::DuplicateNamedTargetDefinition(e) => Some(e.primary_anchor()),
        }
    }

    fn related_anchors(&self) -> Vec<ManifestRelatedAnchor> {
        match self {
            Self::ProfileCairoConflict(e) => e.related_anchors(),
            Self::DependencyGitReferenceAmbiguous(e) => e.related_anchors(),
            Self::DependencyGitPathAmbiguous(e) => e.related_anchors(),
            Self::DependencyGitRegistryAmbiguous(e) => e.related_anchors(),
            Self::PatchSourceConflict(e) => e.related_anchors(),
            _ => vec![],
        }
    }
}

// ── Profile errors ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct ProfileNameInvalid {
    pub name: SmolStr,
    pub message: String,
}

impl ProfileNameInvalid {
    pub fn new(name: impl Into<SmolStr>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            message: message.into(),
        }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        ManifestDiagnosticAnchor::profile(self.name.clone())
    }
}

#[derive(Debug, Clone, Error)]
#[error("profile can inherit from `dev` or `release` only, found `{parent}`")]
pub struct ProfileInheritanceInvalid {
    pub profile: SmolStr,
    pub parent: SmolStr,
}

impl ProfileInheritanceInvalid {
    pub fn new(profile: impl Into<SmolStr>, parent: impl Into<SmolStr>) -> Self {
        Self {
            profile: profile.into(),
            parent: parent.into(),
        }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        ManifestDiagnosticAnchor::profile(self.profile.clone()).with_field("inherits")
    }
}

#[derive(Debug, Clone, Error)]
#[error(
    "inlining-strategy field is set but its effects are overriden by skip-optimizations = true\n\
     if you want to skip compiler optimizations, unset the inlining-strategy or explicitly set it to \"avoid\""
)]
pub struct ProfileCairoConflict {
    pub profile: SmolStr,
}

impl ProfileCairoConflict {
    pub fn new(profile: impl Into<SmolStr>) -> Self {
        Self {
            profile: profile.into(),
        }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        ManifestDiagnosticAnchor::profile(self.profile.clone()).with_field("inlining-strategy")
    }

    fn related_anchors(&self) -> Vec<ManifestRelatedAnchor> {
        vec![ManifestRelatedAnchor {
            message: "value enabling skip-optimizations".to_string(),
            anchor: ManifestDiagnosticAnchor::profile(self.profile.clone())
                .with_field("skip-optimizations"),
        }]
    }
}

// ── Dependency errors ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Error)]
#[error("dependency `{name}` not found in workspace")]
pub struct DependencyWorkspaceNotFound {
    pub name: PackageName,
    pub table: ManifestDependencyTable,
}

impl DependencyWorkspaceNotFound {
    pub fn new(name: PackageName, table: ManifestDependencyTable) -> Self {
        Self { name, table }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        ManifestDiagnosticAnchor::dependency(self.table.clone(), self.name.clone())
    }
}

#[derive(Debug, Clone, Error)]
#[error("dependency ({name}) is non-Git, but provides `branch`, `tag` or `rev`")]
pub struct DependencyGitRefWithoutGit {
    pub name: PackageName,
    pub anchor: ManifestDiagnosticAnchor,
    pub field: &'static str,
}

impl DependencyGitRefWithoutGit {
    pub fn new(name: PackageName, anchor: ManifestDiagnosticAnchor, field: &'static str) -> Self {
        Self {
            name,
            anchor,
            field,
        }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        self.anchor.clone().with_field(self.field)
    }
}

#[derive(Debug, Clone, Error)]
#[error(
    "dependency ({name}) specification is ambiguous, only one of `branch`, `tag` or `rev` is allowed"
)]
pub struct DependencyGitReferenceAmbiguous {
    pub name: PackageName,
    pub anchor: ManifestDiagnosticAnchor,
    pub fields: Vec<&'static str>,
}

impl DependencyGitReferenceAmbiguous {
    pub fn new(
        name: PackageName,
        anchor: ManifestDiagnosticAnchor,
        fields: Vec<&'static str>,
    ) -> Self {
        Self {
            name,
            anchor,
            fields,
        }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        self.anchor
            .clone()
            .with_field(self.fields.first().copied().unwrap_or("branch"))
    }

    fn related_anchors(&self) -> Vec<ManifestRelatedAnchor> {
        self.fields
            .iter()
            .skip(1)
            .map(|field| ManifestRelatedAnchor {
                message: "conflicting Git reference".to_string(),
                anchor: self.anchor.clone().with_field(field),
            })
            .collect()
    }
}

#[derive(Debug, Clone, Error)]
#[error(
    "dependency ({name}) must be specified providing a local path, Git repository, or version to use"
)]
pub struct DependencySourceMissing {
    pub name: PackageName,
    pub anchor: ManifestDiagnosticAnchor,
}

impl DependencySourceMissing {
    pub fn new(name: PackageName, anchor: ManifestDiagnosticAnchor) -> Self {
        Self { name, anchor }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        self.anchor.clone()
    }
}

#[derive(Debug, Clone, Error)]
#[error("dependency ({name}) specification is ambiguous, only one of `git` or `path` is allowed")]
pub struct DependencyGitPathAmbiguous {
    pub name: PackageName,
    pub anchor: ManifestDiagnosticAnchor,
}

impl DependencyGitPathAmbiguous {
    pub fn new(name: PackageName, anchor: ManifestDiagnosticAnchor) -> Self {
        Self { name, anchor }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        self.anchor.clone().with_field("git")
    }

    fn related_anchors(&self) -> Vec<ManifestRelatedAnchor> {
        vec![ManifestRelatedAnchor {
            message: "conflicts with this field".to_string(),
            anchor: self.anchor.clone().with_field("path"),
        }]
    }
}

#[derive(Debug, Clone, Error)]
#[error(
    "dependency ({name}) specification is ambiguous, only one of `git` or `registry` is allowed"
)]
pub struct DependencyGitRegistryAmbiguous {
    pub name: PackageName,
    pub anchor: ManifestDiagnosticAnchor,
}

impl DependencyGitRegistryAmbiguous {
    pub fn new(name: PackageName, anchor: ManifestDiagnosticAnchor) -> Self {
        Self { name, anchor }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        self.anchor.clone().with_field("git")
    }

    fn related_anchors(&self) -> Vec<ManifestRelatedAnchor> {
        vec![ManifestRelatedAnchor {
            message: "conflicts with this field".to_string(),
            anchor: self.anchor.clone().with_field("registry"),
        }]
    }
}

// ── Patch errors ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Error)]
#[error(
    "the `[patch]` section can only be defined in the workspace root manifests\n\
     section found in manifest: `{manifest_path}`\n\
     workspace root manifest: `{workspace_manifest_path}`"
)]
pub struct PatchNotInWorkspaceRoot {
    pub manifest_path: Utf8PathBuf,
    pub workspace_manifest_path: Utf8PathBuf,
}

impl PatchNotInWorkspaceRoot {
    pub fn new(manifest_path: Utf8PathBuf, workspace_manifest_path: Utf8PathBuf) -> Self {
        Self {
            manifest_path,
            workspace_manifest_path,
        }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        ManifestDiagnosticAnchor::patch_root()
    }
}

#[derive(Debug, Clone, Error)]
#[error("the `[patch]` section cannot specify both `{source_a}` and `{source_b}`")]
pub struct PatchSourceConflict {
    pub source_a: SmolStr,
    pub source_b: SmolStr,
}

impl PatchSourceConflict {
    pub fn new(source_a: impl Into<SmolStr>, source_b: impl Into<SmolStr>) -> Self {
        Self {
            source_a: source_a.into(),
            source_b: source_b.into(),
        }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        ManifestDiagnosticAnchor::patch_source(self.source_a.clone())
    }

    fn related_anchors(&self) -> Vec<ManifestRelatedAnchor> {
        vec![ManifestRelatedAnchor {
            message: "conflicts with this source".to_string(),
            anchor: ManifestDiagnosticAnchor::patch_source(self.source_b.clone()),
        }]
    }
}

#[derive(Debug, Clone, Error)]
#[error("failed to parse `{raw_source}` as patch source url")]
pub struct PatchSourceInvalidUrl {
    pub raw_source: SmolStr,
    #[source]
    pub cause: UrlParseError,
}

impl PatchSourceInvalidUrl {
    pub fn new(raw_source: impl Into<SmolStr>, cause: UrlParseError) -> Self {
        Self {
            raw_source: raw_source.into(),
            cause,
        }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        ManifestDiagnosticAnchor::patch_source(self.raw_source.clone())
    }
}

// ── File-path errors ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Error)]
#[error("failed to find readme at {path}")]
pub struct ReadmePathInvalid {
    pub path: Utf8PathBuf,
    pub anchor: Option<ManifestDiagnosticAnchor>,
}

impl ReadmePathInvalid {
    pub fn new(path: Utf8PathBuf, anchor: Option<ManifestDiagnosticAnchor>) -> Self {
        Self { path, anchor }
    }

    fn primary_anchor(&self) -> Option<ManifestDiagnosticAnchor> {
        self.anchor.clone()
    }
}

#[derive(Debug, Clone, Error)]
#[error("failed to find license at {path}")]
pub struct LicensePathInvalid {
    pub path: Utf8PathBuf,
    pub anchor: Option<ManifestDiagnosticAnchor>,
}

impl LicensePathInvalid {
    pub fn new(path: Utf8PathBuf, anchor: Option<ManifestDiagnosticAnchor>) -> Self {
        Self { path, anchor }
    }

    fn primary_anchor(&self) -> Option<ManifestDiagnosticAnchor> {
        self.anchor.clone()
    }
}

// ── Target errors ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Error)]
#[error(
    "manifest contains duplicate target definitions `{kind}`, \
     consider explicitly naming targets with the `name` field"
)]
pub struct DuplicateDefaultTargetDefinition {
    pub kind: TargetKind,
    pub name: SmolStr,
}

impl DuplicateDefaultTargetDefinition {
    pub fn new(kind: TargetKind, name: SmolStr) -> Self {
        Self { kind, name }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        ManifestDiagnosticAnchor::target(self.kind.clone(), Some(self.name.to_string()))
    }
}

#[derive(Debug, Clone, Error)]
#[error(
    "manifest contains duplicate target definitions `{kind} ({name})`, \
     use different target names to resolve the conflict"
)]
pub struct DuplicateNamedTargetDefinition {
    pub kind: TargetKind,
    pub name: SmolStr,
}

impl DuplicateNamedTargetDefinition {
    pub fn new(kind: TargetKind, name: SmolStr) -> Self {
        Self { kind, name }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        ManifestDiagnosticAnchor::target(self.kind.clone(), Some(self.name.to_string()))
    }
}
