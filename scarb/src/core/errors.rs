use thiserror::Error;

#[derive(Debug, Error)]
#[error("script failed with exit code: {exit_code}")]
pub struct ScriptExecutionError {
    /// The process exit code.
    pub exit_code: i32,
}

impl ScriptExecutionError {
    pub fn new(exit_code: i32) -> Self {
        Self { exit_code }
    }
}
