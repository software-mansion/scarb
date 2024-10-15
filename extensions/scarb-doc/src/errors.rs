use core::fmt;
use scarb_metadata::MetadataCommandError as ScarbMetadataCommandFail;
use serde_json::Error as SerdeError;
use std::error::Error as StdError;
use std::io::Error as IOError;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("failed to serialize information about crates")]
pub struct PackagesSerializationError(#[from] SerdeError);

#[derive(Debug, Error)]
#[error("failed to find {0} package")]
pub struct MissingPackageError(pub String);

#[derive(Debug, Error)]
#[error("cairo's `Cfg` must serialize identically as Scarb Metadata's `Cfg`")]
pub struct CfgParseError(#[from] SerdeError);

#[derive(Debug, Error)]
#[error("failed to find corelib")]
pub struct MissingCorelibError;

#[derive(Debug, Error)]
#[error("failed to find compilation unit for package {0}")]
pub struct MissingCompilationUnitForPackage(pub String);

#[derive(Debug, Error)]
#[error("metadata command failed")]
pub struct MetadataCommandError(#[from] ScarbMetadataCommandFail);

#[derive(Debug, Error)]
#[error("could not compile {0} due to previous error")]
pub struct DiagnosticError(pub String);

pub struct IODirectoryCreationError {
    inner_error: IOError,
    directory_purpose: String,
}

impl IODirectoryCreationError {
    pub fn new(err: IOError, directory_purpose: &str) -> Self {
        Self {
            inner_error: err,
            directory_purpose: String::from(directory_purpose),
        }
    }
}

impl fmt::Debug for IODirectoryCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to create directory for {} due to error: {}",
            self.directory_purpose, self.inner_error
        )
    }
}

impl fmt::Display for IODirectoryCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "failed to write content {} due to error: {}",
            self.directory_purpose, self.inner_error
        )
    }
}

impl StdError for IODirectoryCreationError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.inner_error)
    }
}

pub struct IOWriteError {
    inner_error: IOError,
    content_name: String,
}

impl IOWriteError {
    pub fn new(err: IOError, content: &str) -> Self {
        Self {
            inner_error: err,
            content_name: String::from(content),
        }
    }
}

impl fmt::Debug for IOWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to write content {} to a file due to error: {}",
            self.content_name, self.inner_error
        )
    }
}

impl fmt::Display for IOWriteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "failed to write content {} to a file due to error: {}",
            self.content_name, self.inner_error
        )
    }
}

impl StdError for IOWriteError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.inner_error)
    }
}
