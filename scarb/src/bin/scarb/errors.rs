use anyhow::Result;
use thiserror::Error;

/// The ErrorWithExitCode is the error type used at Scarb's CLI-layer.
#[derive(Error, Debug)]
#[error("ErrorWithExitCode exit_code: {exit_code}")]
pub struct ErrorWithExitCode {
    /// The error to display. This can be `None` in rare cases to exit with a
    /// code without displaying a message.
    #[source]
    pub source: Option<anyhow::Error>,
    /// The process exit code.
    pub exit_code: i32,
}

impl ErrorWithExitCode {
    pub fn new(error: anyhow::Error, code: i32) -> Self {
        Self {
            source: Some(error),
            exit_code: code,
        }
    }

    pub fn code(code: i32) -> Self {
        Self {
            source: None,
            exit_code: code,
        }
    }
}

impl From<anyhow::Error> for ErrorWithExitCode {
    fn from(err: anyhow::Error) -> ErrorWithExitCode {
        ErrorWithExitCode::new(err, 1)
    }
}

impl From<std::io::Error> for ErrorWithExitCode {
    fn from(err: std::io::Error) -> ErrorWithExitCode {
        ErrorWithExitCode::new(err.into(), 1)
    }
}

pub fn error_with_exit_code<T>(code: i32) -> Result<T> {
    Err(ErrorWithExitCode::code(code).into())
}
