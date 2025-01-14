use std::vec::IntoIter;

mod conversion;
mod expansions;
mod token;

pub use expansions::*;
pub use token::*;

/// Result of procedural macro code generation.
#[derive(Debug, Clone)]
pub struct ProcMacroResult {
    pub token_stream: TokenStream,
    pub aux_data: Option<AuxData>,
    pub diagnostics: Vec<Diagnostic>,
    pub full_path_markers: Vec<String>,
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
/// use cairo_lang_macro::{AuxData, ProcMacroResult, TokenStream, TokenTree, Token, TextSpan, attribute_macro, post_process, PostProcessContext};
/// use serde::{Serialize, Deserialize};
/// #[derive(Debug, Serialize, Deserialize)]
/// struct SomeAuxDataFormat {
///     some_message: String
/// }
///
/// #[attribute_macro]
/// pub fn some_macro(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
///     // Remove macro call to avoid infinite loop.
///     let code = token_stream.to_string().replace("#[some]", "");
///     let token_stream = TokenStream::new(vec![
///         TokenTree::Ident(
///             Token::new(
///                 code.clone(),
///                 TextSpan::new(0, code.len())
///             )
///         )
///     ]);
///     let value = SomeAuxDataFormat { some_message: "Hello from some macro!".to_string() };
///     let value = serde_json::to_string(&value).unwrap();
///     let value: Vec<u8> = value.into_bytes();
///     let aux_data = AuxData::new(value);
///     ProcMacroResult::new(token_stream).with_aux_data(aux_data)
/// }
///
/// #[post_process]
/// pub fn callback(context: PostProcessContext) {
///     let aux_data = context.aux_data.into_iter()
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

impl From<Diagnostic> for Diagnostics {
    fn from(diagnostics: Diagnostic) -> Self {
        Self(vec![diagnostics])
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

impl FromIterator<Diagnostic> for Diagnostics {
    fn from_iter<T: IntoIterator<Item = Diagnostic>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Extend<Diagnostic> for Diagnostics {
    fn extend<T: IntoIterator<Item = Diagnostic>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl ProcMacroResult {
    /// Create new [`ProcMacroResult`], empty diagnostics set.
    pub fn new(token_stream: TokenStream) -> Self {
        Self {
            token_stream,
            aux_data: Default::default(),
            diagnostics: Default::default(),
            full_path_markers: Default::default(),
        }
    }

    /// Set [`AuxData`] on the [`ProcMacroResult`].
    pub fn with_aux_data(mut self, aux_data: AuxData) -> Self {
        self.aux_data = Some(aux_data);
        self
    }

    /// Append full path markers to the [`ProcMacroResult`].
    pub fn with_full_path_markers(mut self, full_path_markers: Vec<String>) -> Self {
        self.full_path_markers.extend(full_path_markers);
        self
    }

    /// Append diagnostics to the [`ProcMacroResult`] diagnostics set.
    pub fn with_diagnostics(mut self, diagnostics: Diagnostics) -> Self {
        self.diagnostics.extend(diagnostics);
        self
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
        let token_stream = TokenStream::empty();
        assert!(token_stream.metadata.file_id.is_none());
        assert!(token_stream.metadata.original_file_path.is_none());
    }
}
