use crate::compiler::ProfileValidationError;
use crate::core::{PackageName, TargetKind};
use camino::Utf8PathBuf;
use thiserror::Error;
use toml_edit::Table;
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
    CairoInliningStrategyConflict(#[from] CairoInliningStrategyConflict),
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
    /// Resolves this error's anchor(s) to byte spans using the parsed manifest root table.
    pub fn resolve(&self, root: &Table) -> ManifestDiagnosticData {
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
            Self::CairoInliningStrategyConflict(e) => Some(e.primary_anchor()),
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
            Self::CairoInliningStrategyConflict(e) => e.related_anchors(),
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
#[error("{cause}")]
pub struct ProfileNameInvalid {
    pub name: String,
    #[source]
    pub cause: ProfileValidationError,
}

impl ProfileNameInvalid {
    pub fn new(name: impl Into<String>, cause: ProfileValidationError) -> Self {
        Self {
            name: name.into(),
            cause,
        }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        ManifestDiagnosticAnchor::profile(self.name.clone())
    }
}

#[derive(Debug, Clone, Error)]
#[error("profile can inherit from `dev` or `release` only, found `{parent}`")]
pub struct ProfileInheritanceInvalid {
    pub profile: String,
    pub parent: String,
}

impl ProfileInheritanceInvalid {
    pub fn new(profile: impl Into<String>, parent: impl Into<String>) -> Self {
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
    "inlining-strategy field is set but its effects are overridden by skip-optimizations = true\n\
     if you want to skip compiler optimizations, unset the inlining-strategy or explicitly set it to \"avoid\""
)]
pub struct CairoInliningStrategyConflict {
    pub profile: String,
}

impl CairoInliningStrategyConflict {
    pub fn new(profile: impl Into<String>) -> Self {
        Self {
            profile: profile.into(),
        }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        ManifestDiagnosticAnchor::profile(self.profile.clone())
            .with_sub_table("cairo")
            .with_field("inlining-strategy")
    }

    fn related_anchors(&self) -> Vec<ManifestRelatedAnchor> {
        vec![ManifestRelatedAnchor {
            message: "value enabling skip-optimizations".to_string(),
            anchor: ManifestDiagnosticAnchor::profile(self.profile.clone())
                .with_sub_table("cairo")
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
    pub source_a: String,
    pub source_b: String,
}

impl PatchSourceConflict {
    pub fn new(source_a: impl Into<String>, source_b: impl Into<String>) -> Self {
        Self {
            source_a: source_a.into(),
            source_b: source_b.into(),
        }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        ManifestDiagnosticAnchor::patch_source(self.source_a.to_string())
    }

    fn related_anchors(&self) -> Vec<ManifestRelatedAnchor> {
        vec![ManifestRelatedAnchor {
            message: "conflicts with this source".to_string(),
            anchor: ManifestDiagnosticAnchor::patch_source(self.source_b.to_string()),
        }]
    }
}

#[derive(Debug, Clone, Error)]
#[error("failed to parse `{raw_source}` as patch source url")]
pub struct PatchSourceInvalidUrl {
    pub raw_source: String,
    #[source]
    pub cause: UrlParseError,
}

impl PatchSourceInvalidUrl {
    pub fn new(raw_source: impl Into<String>, cause: UrlParseError) -> Self {
        Self {
            raw_source: raw_source.into(),
            cause,
        }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        ManifestDiagnosticAnchor::patch_source(self.raw_source.to_string())
    }
}

// ── File-path errors ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct ReadmePathInvalid {
    pub message: String,
    pub anchor: Option<ManifestDiagnosticAnchor>,
}

impl ReadmePathInvalid {
    pub fn new(message: impl Into<String>, anchor: Option<ManifestDiagnosticAnchor>) -> Self {
        Self {
            message: message.into(),
            anchor,
        }
    }

    fn primary_anchor(&self) -> Option<ManifestDiagnosticAnchor> {
        self.anchor.clone()
    }
}

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct LicensePathInvalid {
    pub message: String,
    pub anchor: Option<ManifestDiagnosticAnchor>,
}

impl LicensePathInvalid {
    pub fn new(message: impl Into<String>, anchor: Option<ManifestDiagnosticAnchor>) -> Self {
        Self {
            message: message.into(),
            anchor,
        }
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
    pub name: String,
}

impl DuplicateDefaultTargetDefinition {
    pub fn new(kind: TargetKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
        }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        ManifestDiagnosticAnchor::target(self.kind.clone(), self.name.clone())
    }
}

#[derive(Debug, Clone, Error)]
#[error(
    "manifest contains duplicate target definitions `{kind} ({name})`, \
     use different target names to resolve the conflict"
)]
pub struct DuplicateNamedTargetDefinition {
    pub kind: TargetKind,
    pub name: String,
}

impl DuplicateNamedTargetDefinition {
    pub fn new(kind: TargetKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
        }
    }

    fn primary_anchor(&self) -> ManifestDiagnosticAnchor {
        ManifestDiagnosticAnchor::target(self.kind.clone(), self.name.clone())
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;
    use crate::compiler::ProfileValidationError;
    use crate::core::manifest::ManifestDiagnosticSpan;

    /// Parses `toml`, calls `resolve` on `err`, and returns the diagnostic data.
    fn resolve_err(err: impl Into<ManifestSemanticError>, toml: &str) -> ManifestDiagnosticData {
        let doc = toml_edit::Document::parse(toml).expect("valid TOML");
        err.into().resolve(doc.as_table())
    }

    /// Slices `source` at `span`, panicking if `span` is `None`.
    fn span_text(source: &str, span: Option<ManifestDiagnosticSpan>) -> &str {
        let span = span.expect("expected Some span, got None");
        &source[span.start..span.end]
    }

    // ── Profile errors ────────────────────────────────────────────────────────

    #[test]
    fn profile_name_invalid_anchors_to_profile_section() {
        let toml = indoc! {r#"
            [profile.bad_name]
        "#};
        let err = ProfileNameInvalid::new(
            "bad_name",
            ProfileValidationError::InvalidCharacters {
                name: "bad_name".into(),
            },
        );
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "[profile.bad_name]");
    }

    #[test]
    fn profile_inheritance_invalid_anchors_to_inherits_field() {
        let toml = indoc! {r#"
            [profile.custom]
            inherits = "invalid"
        "#};
        let err = ProfileInheritanceInvalid::new("custom", "invalid");
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "inherits");
    }

    #[test]
    fn cairo_inlining_strategy_conflict_anchors_to_both_fields() {
        let toml = indoc! {r#"
            [profile.release.cairo]
            inlining-strategy = "inline_all"
            skip-optimizations = true
        "#};
        let err = CairoInliningStrategyConflict::new("release");
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "inlining-strategy");
        assert_eq!(data.related.len(), 1);
        assert_eq!(
            &toml[data.related[0].span.start..data.related[0].span.end],
            "skip-optimizations"
        );
    }

    // ── Dependency errors ─────────────────────────────────────────────────────

    #[test]
    fn dependency_workspace_not_found_anchors_to_dep_key() {
        let toml = indoc! {r#"
            [dependencies]
            foo = { workspace = true }
        "#};
        let err = DependencyWorkspaceNotFound::new(
            PackageName::new("foo"),
            ManifestDependencyTable::Dependencies,
        );
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "foo");
        assert!(data.related.is_empty());
    }

    #[test]
    fn dependency_workspace_not_found_dev_dep_anchors_to_dep_key() {
        let toml = indoc! {r#"
            [dev-dependencies]
            foo = { workspace = true }
        "#};
        let err = DependencyWorkspaceNotFound::new(
            PackageName::new("foo"),
            ManifestDependencyTable::DevDependencies,
        );
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "foo");
        assert!(data.related.is_empty());
    }

    #[test]
    fn dependency_workspace_not_found_workspace_dep_anchors_to_dep_key() {
        let toml = indoc! {r#"
            [workspace.dependencies]
            foo = { workspace = true }
        "#};
        let err = DependencyWorkspaceNotFound::new(
            PackageName::new("foo"),
            ManifestDependencyTable::WorkspaceDependencies,
        );
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "foo");
        assert!(data.related.is_empty());
    }

    #[test]
    fn dependency_git_ref_without_git_anchors_to_branch_field() {
        let toml = indoc! {r#"
            [dependencies]
            foo = { path = "../foo", branch = "main" }
        "#};
        let anchor = ManifestDiagnosticAnchor::dependency(
            ManifestDependencyTable::Dependencies,
            PackageName::new("foo"),
        );
        let err = DependencyGitRefWithoutGit::new(PackageName::new("foo"), anchor, "branch");
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "branch");
        assert!(data.related.is_empty());
    }

    #[test]
    fn dependency_git_ref_without_git_anchors_to_rev_field() {
        let toml = indoc! {r#"
            [dependencies]
            foo = { path = "../foo", rev = "abc123" }
        "#};
        let anchor = ManifestDiagnosticAnchor::dependency(
            ManifestDependencyTable::Dependencies,
            PackageName::new("foo"),
        );
        let err = DependencyGitRefWithoutGit::new(PackageName::new("foo"), anchor, "rev");
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "rev");
        assert!(data.related.is_empty());
    }

    #[test]
    fn dependency_git_ref_without_git_anchors_to_tag_field() {
        let toml = indoc! {r#"
            [dependencies]
            foo = { path = "../foo", tag = "v1.0" }
        "#};
        let anchor = ManifestDiagnosticAnchor::dependency(
            ManifestDependencyTable::Dependencies,
            PackageName::new("foo"),
        );
        let err = DependencyGitRefWithoutGit::new(PackageName::new("foo"), anchor, "tag");
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "tag");
        assert!(data.related.is_empty());
    }

    #[test]
    fn dependency_git_reference_ambiguous_anchors_to_all_ref_fields() {
        let toml = indoc! {r#"
            [dependencies]
            foo = { git = "https://example.com", branch = "main", tag = "v1" }
        "#};
        let anchor = ManifestDiagnosticAnchor::dependency(
            ManifestDependencyTable::Dependencies,
            PackageName::new("foo"),
        );
        let err = DependencyGitReferenceAmbiguous::new(
            PackageName::new("foo"),
            anchor,
            vec!["branch", "tag"],
        );
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "branch");
        assert_eq!(data.related.len(), 1);
        assert_eq!(
            &toml[data.related[0].span.start..data.related[0].span.end],
            "tag"
        );
    }

    #[test]
    fn dependency_source_missing_anchors_to_dep_key() {
        let toml = indoc! {r#"
            [dependencies]
            foo = {}
        "#};
        let anchor = ManifestDiagnosticAnchor::dependency(
            ManifestDependencyTable::Dependencies,
            PackageName::new("foo"),
        );
        let err = DependencySourceMissing::new(PackageName::new("foo"), anchor);
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "foo");
    }

    #[test]
    fn dependency_git_path_ambiguous_anchors_to_both_fields() {
        let toml = indoc! {r#"
            [dependencies]
            foo = { git = "https://example.com", path = "../foo" }
        "#};
        let anchor = ManifestDiagnosticAnchor::dependency(
            ManifestDependencyTable::Dependencies,
            PackageName::new("foo"),
        );
        let err = DependencyGitPathAmbiguous::new(PackageName::new("foo"), anchor);
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "git");
        assert_eq!(data.related.len(), 1);
        assert_eq!(
            &toml[data.related[0].span.start..data.related[0].span.end],
            "path"
        );
    }

    #[test]
    fn dependency_git_registry_ambiguous_anchors_to_both_fields() {
        let toml = indoc! {r#"
            [dependencies]
            foo = { git = "https://example.com", registry = "custom" }
        "#};
        let anchor = ManifestDiagnosticAnchor::dependency(
            ManifestDependencyTable::Dependencies,
            PackageName::new("foo"),
        );
        let err = DependencyGitRegistryAmbiguous::new(PackageName::new("foo"), anchor);
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "git");
        assert_eq!(data.related.len(), 1);
        assert_eq!(
            &toml[data.related[0].span.start..data.related[0].span.end],
            "registry"
        );
    }

    // ── Patch errors ──────────────────────────────────────────────────────────

    #[test]
    fn patch_not_in_workspace_root_anchors_to_patch_section() {
        let toml = indoc! {r#"
            [patch.crates-io]
        "#};
        let err = PatchNotInWorkspaceRoot::new(
            camino::Utf8PathBuf::from("member/Scarb.toml"),
            camino::Utf8PathBuf::from("Scarb.toml"),
        );
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "[patch.crates-io]");
    }

    #[test]
    fn patch_source_conflict_anchors_to_both_sources() {
        let toml = indoc! {r#"
            [patch.crates-io]
            [patch.my-registry]
        "#};
        let err = PatchSourceConflict::new("crates-io", "my-registry");
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "[patch.crates-io]");
        assert_eq!(data.related.len(), 1);
        assert_eq!(
            &toml[data.related[0].span.start..data.related[0].span.end],
            "[patch.my-registry]"
        );
    }

    #[test]
    fn patch_source_invalid_url_anchors_to_source_table() {
        let toml = indoc! {r#"
            [patch.not-a-url]
        "#};
        let cause = url::Url::parse("not-a-url").unwrap_err();
        let err = PatchSourceInvalidUrl::new("not-a-url", cause);
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "[patch.not-a-url]");
    }

    // ── File-path errors ──────────────────────────────────────────────────────

    #[test]
    fn readme_path_invalid_anchors_to_readme_field() {
        let toml = indoc! {r#"
            [package]
            readme = "MISSING.md"
        "#};
        let anchor = ManifestDiagnosticAnchor::package_field("readme");
        let err = ReadmePathInvalid::new("readme not found", Some(anchor));
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "readme");
    }

    #[test]
    fn license_path_invalid_anchors_to_license_file_field() {
        let toml = indoc! {r#"
            [package]
            license-file = "MISSING.txt"
        "#};
        let anchor = ManifestDiagnosticAnchor::package_field("license-file");
        let err = LicensePathInvalid::new("license file not found", Some(anchor));
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "license-file");
    }

    // ── Target errors ─────────────────────────────────────────────────────────

    #[test]
    fn duplicate_default_target_definition_anchors_to_target_section() {
        let toml = indoc! {r#"
            [lib]
            name = "lib"
        "#};
        let err = DuplicateDefaultTargetDefinition::new(TargetKind::LIB, "lib");
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "name");
    }

    #[test]
    fn duplicate_named_target_definition_anchors_to_name_field() {
        let toml = indoc! {r#"
            [[target.starknet-contract]]
            name = "mycontract"
        "#};
        let err = DuplicateNamedTargetDefinition::new(TargetKind::STARKNET_CONTRACT, "mycontract");
        let data = resolve_err(err, toml);
        assert_eq!(span_text(toml, data.span), "name");
    }
}
