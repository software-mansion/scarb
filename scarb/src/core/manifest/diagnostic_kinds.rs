use thiserror::Error;

use super::ManifestDiagnosticData;

/// Typed manifest validation errors that carry semantic anchors for diagnostic span resolution.
///
/// This enum is the dispatch point for all manifest validation errors that can be anchored
/// to a specific location in the TOML source. New error types are added as variants here
/// in subsequent commits.
#[derive(Debug, Clone, Error)]
pub enum ManifestSemanticError {}

impl ManifestSemanticError {
    /// Resolves this error's anchor(s) to byte spans using the raw manifest source.
    pub fn resolve(&self, _manifest_source: &str) -> ManifestDiagnosticData {
        match *self {}
    }
}
