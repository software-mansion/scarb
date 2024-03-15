use std::fmt::Display;
use std::vec::IntoIter;

mod conversion;
mod expansions;

pub use expansions::*;

#[derive(Debug)]
pub enum ProcMacroResult {
    /// Plugin has not taken any action.
    Leave { diagnostics: Vec<Diagnostic> },
    /// Plugin generated [`TokenStream`] replacement.
    Replace {
        token_stream: TokenStream,
        aux_data: Option<AuxData>,
        diagnostics: Vec<Diagnostic>,
    },
    /// Plugin ordered item removal.
    Remove { diagnostics: Vec<Diagnostic> },
}

#[derive(Debug, Default, Clone)]
pub struct TokenStream {
    value: String,
    metadata: TokenStreamMetadata,
}

#[derive(Debug, Default, Clone)]
pub struct TokenStreamMetadata {
    original_file_path: Option<String>,
    file_id: Option<String>,
}

impl TokenStream {
    #[doc(hidden)]
    pub fn new(value: String) -> Self {
        Self {
            value,
            metadata: TokenStreamMetadata::default(),
        }
    }

    #[doc(hidden)]
    pub fn with_metadata(mut self, metadata: TokenStreamMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn metadata(&self) -> &TokenStreamMetadata {
        &self.metadata
    }
}

impl Display for TokenStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl TokenStreamMetadata {
    pub fn new(file_path: impl ToString, file_id: impl ToString) -> Self {
        Self {
            original_file_path: Some(file_path.to_string()),
            file_id: Some(file_id.to_string()),
        }
    }
}

/// Auxiliary data returned by procedural macro.
#[derive(Debug, Clone)]
pub struct AuxData(Vec<u8>);

impl AuxData {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }
}

impl From<&[u8]> for AuxData {
    fn from(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }
}

impl From<AuxData> for Vec<u8> {
    fn from(aux_data: AuxData) -> Vec<u8> {
        aux_data.0
    }
}

/// Diagnostic returned by the procedural macro.
#[derive(Debug)]
pub struct Diagnostic {
    pub message: String,
    pub severity: Severity,
}

/// The severity of a diagnostic.
#[derive(Debug)]
pub enum Severity {
    Error = 1,
    Warning = 2,
}

#[derive(Debug)]
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostic {
    pub fn error(message: impl ToString) -> Self {
        Self {
            message: message.to_string(),
            severity: Severity::Error,
        }
    }

    pub fn warn(message: impl ToString) -> Self {
        Self {
            message: message.to_string(),
            severity: Severity::Warning,
        }
    }
}

impl From<Vec<Diagnostic>> for Diagnostics {
    fn from(diagnostics: Vec<Diagnostic>) -> Self {
        Self(diagnostics)
    }
}
impl Diagnostics {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self(diagnostics)
    }

    pub fn error(mut self, message: impl ToString) -> Self {
        self.0.push(Diagnostic::error(message));
        self
    }

    pub fn warn(mut self, message: impl ToString) -> Self {
        self.0.push(Diagnostic::warn(message));
        self
    }
}

impl IntoIterator for Diagnostics {
    type Item = Diagnostic;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> IntoIter<Diagnostic> {
        self.0.into_iter()
    }
}

impl ProcMacroResult {
    pub fn leave() -> Self {
        Self::Leave {
            diagnostics: Vec::new(),
        }
    }

    pub fn remove() -> Self {
        Self::Remove {
            diagnostics: Vec::new(),
        }
    }

    pub fn replace(token_stream: TokenStream, aux_data: Option<AuxData>) -> Self {
        Self::Replace {
            aux_data,
            token_stream,
            diagnostics: Vec::new(),
        }
    }

    pub fn with_diagnostics(mut self, diagnostics: Diagnostics) -> Self {
        match &mut self {
            Self::Leave { diagnostics: d } => d.extend(diagnostics),
            Self::Remove { diagnostics: d } => d.extend(diagnostics),
            Self::Replace { diagnostics: d, .. } => d.extend(diagnostics),
        };
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::types::TokenStream;

    #[test]
    fn new_token_stream_metadata_empty() {
        let token_stream = TokenStream::new("".to_string());
        assert!(token_stream.metadata.file_id.is_none());
        assert!(token_stream.metadata.original_file_path.is_none());
    }
}
