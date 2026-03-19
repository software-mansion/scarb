use smol_str::SmolStr;
use thiserror::Error;
use toml_edit::Document;

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
    ProfileCairoConflict(#[from] ProfileCairoConflict),
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
        }
    }

    fn related_anchors(&self) -> Vec<ManifestRelatedAnchor> {
        match self {
            Self::ProfileCairoConflict(e) => e.related_anchors(),
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
