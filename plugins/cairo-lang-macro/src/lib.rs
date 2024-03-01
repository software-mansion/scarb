use std::ffi::{c_char, CString};
use std::fmt::Display;

pub use cairo_lang_macro_attributes::*;
use cairo_lang_macro_stable::{StableAuxData, StableProcMacroResult, StableTokenStream};

#[derive(Debug)]
pub enum ProcMacroResult {
    /// Plugin has not taken any action.
    Leave,
    /// Plugin generated [`TokenStream`] replacement.
    Replace {
        token_stream: TokenStream,
        aux_data: Option<AuxData>,
    },
    /// Plugin ordered item removal.
    Remove,
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

impl ProcMacroResult {
    /// Convert to FFI-safe representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub fn into_stable(self) -> StableProcMacroResult {
        match self {
            ProcMacroResult::Leave => StableProcMacroResult::Leave,
            ProcMacroResult::Remove => StableProcMacroResult::Remove,
            ProcMacroResult::Replace {
                token_stream,
                aux_data,
            } => StableProcMacroResult::Replace {
                token_stream: token_stream.into_stable(),
                aux_data: AuxData::maybe_into_stable(aux_data),
            },
        }
    }

    /// Convert to native Rust representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(result: StableProcMacroResult) -> Self {
        match result {
            StableProcMacroResult::Leave => ProcMacroResult::Leave,
            StableProcMacroResult::Remove => ProcMacroResult::Remove,
            StableProcMacroResult::Replace {
                token_stream,
                aux_data,
            } => ProcMacroResult::Replace {
                token_stream: TokenStream::from_stable(token_stream),
                aux_data: AuxData::from_stable(aux_data).unwrap(),
            },
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

unsafe fn raw_to_string(raw: *mut c_char) -> String {
    if raw.is_null() {
        String::default()
    } else {
        let cstr = CString::from_raw(raw);
        cstr.to_string_lossy().to_string()
    }
}
