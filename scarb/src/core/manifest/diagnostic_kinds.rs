use crate::compiler::ProfileValidationError;
use thiserror::Error;
use toml_edit::Table;

use super::ManifestDiagnosticData;
use super::diagnostic::resolve_anchor_in_doc;
use super::{ManifestDiagnosticAnchor, ManifestRelatedAnchor, ManifestRelatedLocation};

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
        }
    }

    fn related_anchors(&self) -> Vec<ManifestRelatedAnchor> {
        match self {
            Self::CairoInliningStrategyConflict(e) => e.related_anchors(),
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
