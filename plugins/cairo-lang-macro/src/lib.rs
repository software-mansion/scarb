pub use cairo_lang_macro_attributes::*;
use cairo_lang_macro_stable::ffi::StableSlice;
use cairo_lang_macro_stable::{
    StableAuxData, StableDiagnostic, StableProcMacroResult, StableSeverity, StableTokenStream,
};
use std::ffi::{c_char, CStr, CString};
use std::fmt::Display;
use std::num::NonZeroU8;
use std::slice;
use std::vec::IntoIter;

/// Free the memory allocated for the [`StableProcMacroResult`].
///
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn free_result(result: StableProcMacroResult) {
    ProcMacroResult::from_owned_stable(result);
}

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
pub struct TokenStream(String);

impl TokenStream {
    #[doc(hidden)]
    pub fn new(s: String) -> Self {
        Self(s)
    }
}

impl Display for TokenStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Auxiliary data returned by procedural macro.
#[derive(Debug)]
pub struct AuxData(String);

impl AuxData {
    pub fn new(s: String) -> Self {
        Self(s)
    }

    pub fn try_new<T: serde::Serialize>(value: T) -> Result<Self, serde_json::Error> {
        Ok(Self(serde_json::to_string(&value)?))
    }
}

impl Display for AuxData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

    /// Convert to FFI-safe representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub fn into_stable(self) -> StableProcMacroResult {
        match self {
            ProcMacroResult::Leave { diagnostics } => {
                let diagnostics = diagnostics
                    .into_iter()
                    .map(|d| d.into_stable())
                    .collect::<Vec<_>>();
                StableProcMacroResult::Leave {
                    diagnostics: StableSlice::new(diagnostics),
                }
            }
            ProcMacroResult::Remove { diagnostics } => {
                let diagnostics = diagnostics
                    .into_iter()
                    .map(|d| d.into_stable())
                    .collect::<Vec<_>>();
                StableProcMacroResult::Remove {
                    diagnostics: StableSlice::new(diagnostics),
                }
            }
            ProcMacroResult::Replace {
                token_stream,
                aux_data,
                diagnostics,
            } => {
                let diagnostics = diagnostics
                    .into_iter()
                    .map(|d| d.into_stable())
                    .collect::<Vec<_>>();
                StableProcMacroResult::Replace {
                    token_stream: token_stream.into_stable(),
                    aux_data: AuxData::maybe_into_stable(aux_data),
                    diagnostics: StableSlice::new(diagnostics),
                }
            }
        }
    }

    /// Convert to native Rust representation, without taking the ownership of the string.
    ///
    /// Note that you still need to free the memory by calling `from_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(result: &StableProcMacroResult) -> Self {
        match result {
            StableProcMacroResult::Leave { diagnostics } => {
                let (ptr, n) = diagnostics.raw_parts();
                let diagnostics = slice::from_raw_parts(ptr, n)
                    .iter()
                    .map(|d| Diagnostic::from_stable(d))
                    .collect::<Vec<_>>();
                ProcMacroResult::Leave { diagnostics }
            }
            StableProcMacroResult::Remove { diagnostics } => {
                let (ptr, n) = diagnostics.raw_parts();
                let diagnostics = slice::from_raw_parts(ptr, n)
                    .iter()
                    .map(|d| Diagnostic::from_stable(d))
                    .collect::<Vec<_>>();
                ProcMacroResult::Remove { diagnostics }
            }
            StableProcMacroResult::Replace {
                token_stream,
                aux_data,
                diagnostics,
            } => {
                let (ptr, n) = diagnostics.raw_parts();
                let diagnostics = slice::from_raw_parts(ptr, n)
                    .iter()
                    .map(|d| Diagnostic::from_stable(d))
                    .collect::<Vec<_>>();
                ProcMacroResult::Replace {
                    token_stream: TokenStream::from_stable(token_stream),
                    aux_data: AuxData::from_stable(aux_data),
                    diagnostics,
                }
            }
        }
    }

    /// Convert to native Rust representation, with taking the ownership of the string.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_owned_stable(result: StableProcMacroResult) -> Self {
        match result {
            StableProcMacroResult::Leave { diagnostics } => {
                let diagnostics = diagnostics.into_owned();
                let diagnostics = diagnostics
                    .into_iter()
                    .map(|d| Diagnostic::from_owned_stable(d))
                    .collect::<Vec<_>>();
                ProcMacroResult::Leave { diagnostics }
            }
            StableProcMacroResult::Remove { diagnostics } => {
                let diagnostics = diagnostics.into_owned();
                let diagnostics = diagnostics
                    .into_iter()
                    .map(|d| Diagnostic::from_owned_stable(d))
                    .collect::<Vec<_>>();
                ProcMacroResult::Remove { diagnostics }
            }
            StableProcMacroResult::Replace {
                token_stream,
                aux_data,
                diagnostics,
            } => {
                let diagnostics = diagnostics.into_owned();
                let diagnostics = diagnostics
                    .into_iter()
                    .map(|d| Diagnostic::from_owned_stable(d))
                    .collect::<Vec<_>>();
                ProcMacroResult::Replace {
                    token_stream: TokenStream::from_owned_stable(token_stream),
                    aux_data: AuxData::from_owned_stable(aux_data),
                    diagnostics,
                }
            }
        }
    }
}

impl TokenStream {
    /// Convert to FFI-safe representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub fn into_stable(self) -> StableTokenStream {
        let cstr = CString::new(self.0).unwrap();
        StableTokenStream::new(cstr.into_raw())
    }

    /// Convert to native Rust representation, without taking the ownership of the string.
    ///
    /// Note that you still need to free the memory by calling `from_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(token_stream: &StableTokenStream) -> Self {
        Self::new(token_stream.to_string())
    }

    /// Convert to native Rust representation, with taking the ownership of the string.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_owned_stable(token_stream: StableTokenStream) -> Self {
        Self::new(token_stream.into_owned_string())
    }
}

impl AuxData {
    /// Convert to FFI-safe representation.
    ///
    /// # Safety
    pub fn maybe_into_stable(aux_data: Option<Self>) -> StableAuxData {
        if let Some(aux_data) = aux_data {
            aux_data.into_stable()
        } else {
            StableAuxData::None
        }
    }

    /// Convert to FFI-safe representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub fn into_stable(self) -> StableAuxData {
        let cstr = CString::new(self.0.to_string()).unwrap();
        StableAuxData::Some(cstr.into_raw())
    }

    /// Convert to native Rust representation, without taking the ownership of the string.
    ///
    /// Note that you still need to free the memory by calling `from_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(aux_data: &StableAuxData) -> Option<Self> {
        match aux_data {
            StableAuxData::None => None,
            StableAuxData::Some(raw) => Some(Self::new(from_raw_cstr(*raw))),
        }
    }

    /// Convert to native Rust representation, with taking the ownership of the string.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_owned_stable(aux_data: StableAuxData) -> Option<Self> {
        match aux_data {
            StableAuxData::None => None,
            StableAuxData::Some(raw) => Some(Self::new(from_raw_cstring(raw))),
        }
    }
}

impl Diagnostic {
    // Convert to FFI-safe representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub fn into_stable(self) -> StableDiagnostic {
        StableDiagnostic {
            message: CString::new(self.message).unwrap().into_raw(),
            severity: self.severity.into_stable(),
        }
    }

    /// Convert to native Rust representation, without taking the ownership of the string.
    ///
    /// Note that you still need to free the memory by calling `from_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(diagnostic: &StableDiagnostic) -> Self {
        Self {
            message: from_raw_cstr(diagnostic.message),
            severity: Severity::from_stable(&diagnostic.severity),
        }
    }

    /// Convert to native Rust representation, with taking the ownership of the string.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_owned_stable(diagnostic: StableDiagnostic) -> Self {
        Self {
            message: from_raw_cstring(diagnostic.message),
            severity: Severity::from_stable(&diagnostic.severity),
        }
    }
}

impl Severity {
    /// Convert to FFI-safe representation.
    /// # Safety
    ///
    #[doc(hidden)]
    pub fn into_stable(self) -> StableSeverity {
        NonZeroU8::try_from(self as u8).unwrap()
    }

    /// Convert to native Rust representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(severity: &StableSeverity) -> Self {
        if *severity == Self::Error.into_stable() {
            Self::Error
        } else {
            // Note that it defaults to warning for unknown values.
            Self::Warning
        }
    }
}

// Create a string from a raw pointer to a c_char.
// Note that this will free the underlying memory.
unsafe fn from_raw_cstring(raw: *mut c_char) -> String {
    if raw.is_null() {
        String::default()
    } else {
        let cstr = CString::from_raw(raw);
        cstr.to_string_lossy().to_string()
    }
}

// Note that this will not free the underlying memory.
// You still need to free the memory by calling `CString::from_raw`.
unsafe fn from_raw_cstr(raw: *mut c_char) -> String {
    if raw.is_null() {
        String::default()
    } else {
        let cstr = CStr::from_ptr(raw);
        cstr.to_string_lossy().to_string()
    }
}
