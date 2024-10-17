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
