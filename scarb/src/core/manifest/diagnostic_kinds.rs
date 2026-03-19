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
pub enum ManifestSemanticError {}

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
        match *self {}
    }

    fn related_anchors(&self) -> Vec<ManifestRelatedAnchor> {
        match *self {}
    }
}
