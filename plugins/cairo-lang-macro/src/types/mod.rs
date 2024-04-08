use std::fmt::Display;
use std::vec::IntoIter;

mod conversion;
mod expansions;

pub use expansions::*;

/// Result of procedural macro code generation.
#[derive(Debug)]
pub enum ProcMacroResult {
    /// Plugin has not taken any action.
    Leave { diagnostics: Vec<Diagnostic> },
    /// Plugin generated [`TokenStream`] replacement.
    Replace {
        token_stream: TokenStream,
        aux_data: Option<AuxData>,
        diagnostics: Vec<Diagnostic>,
        full_path_markers: Vec<String>,
    },
    /// Plugin ordered item removal.
    Remove { diagnostics: Vec<Diagnostic> },
}

/// Result of inline procedural macro code generation.
///
/// This enum differs from `ProcMacroResult` by not having `Remove` variant.
pub enum InlineProcMacroResult {
    /// Plugin has not taken any action.
    Leave { diagnostics: Vec<Diagnostic> },
    /// Plugin generated [`TokenStream`] replacement.
    Replace {
        token_stream: TokenStream,
        aux_data: Option<AuxData>,
        diagnostics: Vec<Diagnostic>,
    },
}

/// An abstract stream of Cairo tokens.
///
/// This is both input and part of an output of a procedural macro.
#[derive(Debug, Default, Clone)]
pub struct TokenStream {
    value: String,
    metadata: TokenStreamMetadata,
}

/// Metadata of [`TokenStream`].
///
/// This struct can be used to describe the origin of the [`TokenStream`].
#[derive(Debug, Default, Clone)]
pub struct TokenStreamMetadata {
    /// The path to the file from which the [`TokenStream`] has been created.
    pub original_file_path: Option<String>,
    /// ID of the file from which the [`TokenStream`] has been created.
    ///
    /// It is guaranteed, that the `file_id` will be unique for each file.
    pub file_id: Option<String>,
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

    /// Get `[TokenStreamMetadata`] associated with this [`TokenStream`].
    ///
    /// The metadata struct can be used to describe the [`TokenStream`] origin.
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
    #[doc(hidden)]
    pub fn new(file_path: impl ToString, file_id: impl ToString) -> Self {
        Self {
            original_file_path: Some(file_path.to_string()),
            file_id: Some(file_id.to_string()),
        }
    }
}

/// **Auxiliary data** returned by procedural macro code generation.
///
/// This struct can be used to collect additional information from the Cairo source code of
/// compiled project.
/// For instance, you can create a procedural macro that collects some information stored by
/// the Cairo programmer as attributes in the project source code.
///
/// The auxiliary data struct stores `Vec<u8>` leaving the serialization and deserialization
/// of the data as user responsibility. No assumptions regarding the serialization algorithm
/// are made.
///
/// For instance, auxiliary data can be serialized as JSON.
///
/// ```
/// use cairo_lang_macro::{AuxData, ProcMacroResult, TokenStream, attribute_macro, post_process};
/// use serde::{Serialize, Deserialize};
/// #[derive(Debug, Serialize, Deserialize)]
/// struct SomeAuxDataFormat {
///     some_message: String
/// }
///
/// #[attribute_macro]
/// pub fn some_macro(token_stream: TokenStream) -> ProcMacroResult {
///     let token_stream = TokenStream::new(
///         token_stream.to_string()
///         // Remove macro call to avoid infinite loop.
///         .replace("#[some]", "")
///     );
///     let value = SomeAuxDataFormat { some_message: "Hello from some macro!".to_string() };
///     let value = serde_json::to_string(&value).unwrap();
///     let value: Vec<u8> = value.into_bytes();
///     let aux_data = AuxData::new(value);
///     ProcMacroResult::replace(token_stream, Some(aux_data))
/// }
///
/// #[post_process]
/// pub fn callback(aux_data: Vec<AuxData>) {
///     let aux_data = aux_data.into_iter()
///         .map(|aux_data| {
///             let value: Vec<u8> = aux_data.into();
///             let aux_data: SomeAuxDataFormat = serde_json::from_slice(&value).unwrap();
///             aux_data
///         })
///         .collect::<Vec<_>>();
///     println!("{:?}", aux_data);
/// }
/// ```
///
/// All auxiliary data emitted during compilation can be consumed
/// in the `post_process` implementation.
#[derive(Debug, Clone)]
pub struct AuxData(Vec<u8>);

impl AuxData {
    /// Create new [`AuxData`] struct from serialized data.
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
    /// A human addressed message associated with the [`Diagnostic`].
    ///
    /// This message will not be parsed by the compiler,
    /// but rather shown to the user as an explanation.
    pub message: String,
    /// The severity of the [`Diagnostic`].
    ///
    /// Defines how this diagnostic should influence the compilation.
    pub severity: Severity,
}

/// The severity of a diagnostic.
///
/// This should be roughly equivalent to the severity of Cairo diagnostics.
///
/// The appropriate action for each diagnostic kind will be taken by `Scarb`.
#[derive(Debug)]
pub enum Severity {
    /// An error has occurred.
    ///
    /// Emitting diagnostic with [`Severity::Error`] severity will fail the source code compilation.
    Error = 1,
    /// A warning suggestion will be shown to the user.
    ///
    /// Emitting diagnostic with [`Severity::Warning`] severity does not stop the compilation.
    Warning = 2,
}

/// A set of diagnostics that arose during the computation.
#[derive(Debug)]
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostic {
    /// Create new diagnostic with severity [`Severity::Error`].
    pub fn error(message: impl ToString) -> Self {
        Self {
            message: message.to_string(),
            severity: Severity::Error,
        }
    }

    /// Create new diagnostic with severity [`Severity::Warning`].
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
    /// Create new [`Diagnostics`] from a vector of diagnostics.
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self(diagnostics)
    }

    /// Create new diagnostic with severity [`Severity::Error`]
    /// and push to the vector.
    pub fn error(mut self, message: impl ToString) -> Self {
        self.0.push(Diagnostic::error(message));
        self
    }

    /// Create new diagnostic with severity [`Severity::Warning`]
    /// and push to the vector.
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
    /// Create new [`ProcMacroResult::Leave`] variant, empty diagnostics set.
    pub fn leave() -> Self {
        Self::Leave {
            diagnostics: Vec::new(),
        }
    }

    /// Create new [`ProcMacroResult::Remove`] variant, empty diagnostics set.
    pub fn remove() -> Self {
        Self::Remove {
            diagnostics: Vec::new(),
        }
    }

    /// Create new [`ProcMacroResult::Replace`] variant, empty diagnostics set.
    pub fn replace(token_stream: TokenStream, aux_data: Option<AuxData>) -> Self {
        Self::Replace {
            aux_data,
            token_stream,
            diagnostics: Vec::new(),
            full_path_markers: Vec::new(),
        }
    }

    /// Append diagnostics to the [`ProcMacroResult`] diagnostics set.
    pub fn with_diagnostics(mut self, diagnostics: Diagnostics) -> Self {
        match &mut self {
            Self::Leave { diagnostics: d } => d.extend(diagnostics),
            Self::Remove { diagnostics: d } => d.extend(diagnostics),
            Self::Replace { diagnostics: d, .. } => d.extend(diagnostics),
        };
        self
    }
}

impl InlineProcMacroResult {
    /// Create new [`InlineProcMacroResult::Leave`] variant, empty diagnostics set.
    pub fn leave() -> Self {
        Self::Leave {
            diagnostics: Vec::new(),
        }
    }

    /// Create new [`InlineProcMacroResult::Replace`] variant, empty diagnostics set.
    pub fn replace(token_stream: TokenStream, aux_data: Option<AuxData>) -> Self {
        Self::Replace {
            aux_data,
            token_stream,
            diagnostics: Vec::new(),
        }
    }

    /// Append diagnostics to the [`InlineProcMacroResult`] diagnostics set.
    pub fn with_diagnostics(mut self, diagnostics: Diagnostics) -> Self {
        match &mut self {
            Self::Leave { diagnostics: d } => d.extend(diagnostics),
            Self::Replace { diagnostics: d, .. } => d.extend(diagnostics),
        };
        self
    }
}

impl From<InlineProcMacroResult> for ProcMacroResult {
    fn from(result: InlineProcMacroResult) -> Self {
        match result {
            InlineProcMacroResult::Leave { diagnostics } => ProcMacroResult::Leave { diagnostics },
            InlineProcMacroResult::Replace {
                token_stream,
                aux_data,
                diagnostics,
            } => ProcMacroResult::Replace {
                token_stream,
                aux_data,
                diagnostics,
                full_path_markers: Vec::new(),
            },
        }
    }
}

/// Input for the post-process callback.
#[derive(Clone, Debug)]
pub struct PostProcessContext {
    /// Auxiliary data returned by the procedural macro.
    pub aux_data: Vec<AuxData>,
    /// Full path markers resolved by the host.
    pub full_path_markers: Vec<FullPathMarker>,
}

/// Full path marker.
///
/// This contains information about full cairo path resolved by the host, identified by key.
#[derive(Clone, Debug)]
pub struct FullPathMarker {
    /// Key returned by the procedural macro.
    pub key: String,
    /// Full path resolved by the host.
    pub full_path: String,
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
