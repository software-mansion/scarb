use anyhow::Result;
use std::process::ExitCode;
use thiserror::Error;

/// The ErrorWithExitCode is the error type used at Scarb's CLI-layer.
#[derive(Error, Debug)]
#[error("ErrorWithExitCode exit_code: {:?}", exit_code)]
pub struct ErrorWithExitCode {
    /// The error to display. This can be `None` in rare cases to exit with a
    /// code without displaying a message.
    #[source]
    pub source: Option<anyhow::Error>,
    /// The process exit code.
    pub exit_code: ExitCode,
}

impl ErrorWithExitCode {
    pub fn new(error: anyhow::Error, code: ExitCode) -> Self {
        Self {
            source: Some(error),
            exit_code: code,
        }
    }

    pub fn code(code: ExitCode) -> Self {
        Self {
            source: None,
            exit_code: code,
        }
    }
}

impl From<anyhow::Error> for ErrorWithExitCode {
    fn from(err: anyhow::Error) -> ErrorWithExitCode {
        ErrorWithExitCode::new(err, ExitCode::FAILURE)
    }
}

impl From<std::io::Error> for ErrorWithExitCode {
    fn from(err: std::io::Error) -> ErrorWithExitCode {
        ErrorWithExitCode::new(err.into(), ExitCode::FAILURE)
    }
}

pub fn error_with_exit_code<T>(code: ExitCode) -> Result<T> {
    Err(ErrorWithExitCode::code(code).into())
}
