use anyhow::Error as AnyhowError;
use camino::{Utf8Path, Utf8PathBuf};
use std::process::ExitCode;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("script failed with exit code: {:?}", exit_code)]
pub struct ScriptExecutionError {
    /// The process exit code.
    pub exit_code: ExitCode,
}

impl ScriptExecutionError {
    pub fn new(exit_code: ExitCode) -> Self {
        Self { exit_code }
    }
}

/// Wraps a TOML/serde parse failure with the manifest file path.
/// Only constructed in `toml_manifest.rs` when raw TOML cannot be deserialized.
#[derive(Debug, Error)]
#[error("failed to parse manifest at: {path}")]
pub struct ManifestParseError {
    path: Utf8PathBuf,
    #[source]
    source: AnyhowError,
}

impl ManifestParseError {
    pub fn new(path: impl Into<Utf8PathBuf>, source: impl Into<AnyhowError>) -> Self {
        Self {
            path: path.into(),
            source: source.into(),
        }
    }

    pub fn path(&self) -> &Utf8Path {
        &self.path
    }
}

/// Carries the raw manifest source text alongside a semantic validation error.
///
/// Constructed in `workspace.rs` once the file has been read, so that the
/// diagnostic emitter can call [`crate::core::ManifestSemanticError::resolve`]
/// with the text without re-reading the file.
#[derive(Debug, Error)]
#[error("failed to parse manifest at: {path}")]
pub struct ManifestErrorWithSource {
    pub path: Utf8PathBuf,
    pub content: String,
    #[source]
    inner: AnyhowError,
}

impl ManifestErrorWithSource {
    pub fn new(
        path: impl Into<Utf8PathBuf>,
        content: impl Into<String>,
        inner: AnyhowError,
    ) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
            inner,
        }
    }
}
