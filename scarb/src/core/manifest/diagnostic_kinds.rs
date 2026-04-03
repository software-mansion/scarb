use crate::compiler::ProfileValidationError;
use crate::core::PackageName;
use thiserror::Error;
use toml_edit::Table;

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
        }
    }

    fn related_anchors(&self) -> Vec<ManifestRelatedAnchor> {
        match self {
            Self::CairoInliningStrategyConflict(e) => e.related_anchors(),
            Self::DependencyGitReferenceAmbiguous(e) => e.related_anchors(),
            Self::DependencyGitPathAmbiguous(e) => e.related_anchors(),
            Self::DependencyGitRegistryAmbiguous(e) => e.related_anchors(),
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
