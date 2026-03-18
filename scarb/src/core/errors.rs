use anyhow::Error as AnyhowError;
use camino::{Utf8Path, Utf8PathBuf};
use std::process::ExitCode;
use thiserror::Error;

use crate::core::manifest::ManifestDiagnosticData;

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

#[derive(Debug, Error)]
#[error("failed to parse manifest at: {path}")]
pub struct ManifestParseError {
    path: Utf8PathBuf,
    diagnostic: Option<ManifestDiagnosticData>,
    #[source]
    source: AnyhowError,
}

impl ManifestParseError {
    pub fn new(path: impl Into<Utf8PathBuf>, source: impl Into<AnyhowError>) -> Self {
        Self {
            path: path.into(),
            diagnostic: None,
            source: source.into(),
        }
    }

    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    pub fn with_diagnostic(mut self, diagnostic: ManifestDiagnosticData) -> Self {
        self.diagnostic = Some(diagnostic);
        self
    }

    pub fn diagnostic(&self) -> Option<&ManifestDiagnosticData> {
        self.diagnostic.as_ref()
    }
}
