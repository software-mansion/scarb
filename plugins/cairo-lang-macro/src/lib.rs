use libc::{free, malloc};
use serde_json::Value;
use std::ffi::{c_char, c_void, CString};
use std::fmt::Display;

pub use cairo_lang_macro_attributes::*;
use cairo_lang_macro_stable::{
    StableAuxData, StableDiagnostic, StableProcMacroResult, StableSeverity, StableTokenStream,
};

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
pub struct AuxData(Value);

impl AuxData {
    pub fn try_new<T: serde::Serialize>(value: T) -> Result<Self, serde_json::Error> {
        Ok(Self(serde_json::to_value(value)?))
    }

    pub fn to_value(self) -> Value {
        self.0
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
    Error,
    Warning,
}

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

impl ProcMacroResult {
    /// Convert to FFI-safe representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub fn into_stable(self) -> StableProcMacroResult {
        match self {
            ProcMacroResult::Leave { diagnostics } => {
                let (ptr, n) = unsafe { Diagnostic::allocate(diagnostics) };
                StableProcMacroResult::Leave {
                    diagnostics: ptr,
                    diagnostics_n: n,
                }
            }
            ProcMacroResult::Remove { diagnostics } => {
                let (ptr, n) = unsafe { Diagnostic::allocate(diagnostics) };
                StableProcMacroResult::Remove {
                    diagnostics: ptr,
                    diagnostics_n: n,
                }
            }
            ProcMacroResult::Replace {
                token_stream,
                aux_data,
                diagnostics,
            } => {
                let (ptr, n) = unsafe { Diagnostic::allocate(diagnostics) };
                StableProcMacroResult::Replace {
                    token_stream: token_stream.into_stable(),
                    aux_data: AuxData::maybe_into_stable(aux_data),
                    diagnostics: ptr,
                    diagnostics_n: n,
                }
            }
        }
    }

    /// Convert to native Rust representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(result: StableProcMacroResult) -> Self {
        match result {
            StableProcMacroResult::Leave {
                diagnostics,
                diagnostics_n,
            } => {
                let diagnostics = Diagnostic::deallocate(diagnostics, diagnostics_n);
                ProcMacroResult::Leave { diagnostics }
            }
            StableProcMacroResult::Remove {
                diagnostics,
                diagnostics_n,
            } => {
                let diagnostics = Diagnostic::deallocate(diagnostics, diagnostics_n);
                ProcMacroResult::Remove { diagnostics }
            }
            StableProcMacroResult::Replace {
                token_stream,
                aux_data,
                diagnostics,
                diagnostics_n,
            } => {
                let diagnostics = Diagnostic::deallocate(diagnostics, diagnostics_n);
                ProcMacroResult::Replace {
                    token_stream: TokenStream::from_stable(token_stream),
                    aux_data: AuxData::from_stable(aux_data).unwrap(),
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
        StableTokenStream(cstr.into_raw())
    }

    /// Convert to native Rust representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(token_stream: StableTokenStream) -> Self {
        Self::new(token_stream.to_string())
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

    /// Convert to native Rust representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(aux_data: StableAuxData) -> Result<Option<Self>, serde_json::Error> {
        match aux_data {
            StableAuxData::None => Ok(None),
            StableAuxData::Some(raw) => Some(Self::try_new(raw_to_string(raw))).transpose(),
        }
    }
}

impl Diagnostic {
    /// Convert to FFI-safe representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub fn into_stable(self) -> StableDiagnostic {
        let cstr = CString::new(self.message).unwrap();
        StableDiagnostic {
            message: cstr.into_raw(),
            severity: self.severity.into_stable(),
        }
    }

    /// Convert to native Rust representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(diagnostic: StableDiagnostic) -> Self {
        Self {
            message: raw_to_string(diagnostic.message),
            severity: Severity::from_stable(diagnostic.severity),
        }
    }

    /// Allocate dynamic array with FFI-safe diagnostics.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn allocate(diagnostics: Vec<Self>) -> (*mut StableDiagnostic, usize) {
        let stable_diagnostics = diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.into_stable())
            .collect::<Vec<_>>();
        let n = stable_diagnostics.len();
        let ptr = malloc(std::mem::size_of::<StableDiagnostic>() * n) as *mut StableDiagnostic;
        if ptr.is_null() {
            panic!("memory allocation with malloc failed");
        }
        for (i, diag) in stable_diagnostics.into_iter().enumerate() {
            let ptr = ptr.add(i);
            std::ptr::write(ptr, diag);
        }
        (ptr, n)
    }

    /// Deallocate dynamic array of diagnostics, returning a vector.
    ///
    /// # Safety
    pub unsafe fn deallocate(ptr: *mut StableDiagnostic, n: usize) -> Vec<Diagnostic> {
        let mut diagnostics: Vec<Diagnostic> = Vec::with_capacity(n);
        for i in 0..n {
            let ptr = ptr.add(i);
            let diag = std::ptr::read(ptr);
            let diag = Diagnostic::from_stable(diag);
            diagnostics.push(diag);
        }
        free(ptr as *mut c_void);
        diagnostics
    }
}

impl Severity {
    /// Convert to FFI-safe representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub fn into_stable(self) -> StableSeverity {
        match self {
            Severity::Error => StableSeverity::Error,
            Severity::Warning => StableSeverity::Warning,
        }
    }

    /// Convert to native Rust representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(severity: StableSeverity) -> Self {
        match severity {
            StableSeverity::Error => Self::Error,
            StableSeverity::Warning => Self::Warning,
        }
    }
}

unsafe fn raw_to_string(raw: *mut c_char) -> String {
    if raw.is_null() {
        String::default()
    } else {
        let cstr = CString::from_raw(raw);
        cstr.to_string_lossy().to_string()
    }
}
