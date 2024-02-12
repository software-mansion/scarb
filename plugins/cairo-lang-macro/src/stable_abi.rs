use crate::{AuxData, ProcMacroResult, TokenStream};
use std::ffi::CString;
use std::os::raw::c_char;

/// Token stream.
///
/// This struct implements FFI-safe stable ABI.
#[repr(C)]
#[derive(Debug)]
pub struct StableTokenStream(pub *mut c_char);

/// Auxiliary data returned by procedural macro.
///
/// This struct implements FFI-safe stable ABI.
#[repr(C)]
#[derive(Debug)]
pub enum StableAuxData {
    None,
    Some(*mut c_char),
}

/// Procedural macro result.
///
/// This struct implements FFI-safe stable ABI.
#[repr(C)]
#[derive(Debug)]
pub enum StableProcMacroResult {
    /// Plugin has not taken any action.
    Leave,
    /// Plugin generated [`TokenStream`] replacement.
    Replace {
        token_stream: StableTokenStream,
        aux_data: StableAuxData,
    },
    /// Plugin ordered item removal.
    Remove,
}

impl StableTokenStream {
    /// Convert to String.
    ///
    /// # Safety
    pub unsafe fn to_string(&self) -> String {
        raw_to_string(self.0)
    }

    /// Convert to native Rust representation.
    ///
    /// # Safety
    pub unsafe fn into_token_stream(self) -> TokenStream {
        TokenStream::new(self.to_string())
    }

    /// Convert to FFI-safe representation.
    ///
    /// # Safety
    pub unsafe fn from_token_stream(token_stream: TokenStream) -> Self {
        let cstr = CString::new(token_stream.0).unwrap();
        StableTokenStream(cstr.into_raw())
    }
}

impl StableAuxData {
    pub unsafe fn into_aux_data(self) -> Result<Option<AuxData>, serde_json::Error> {
        match self {
            Self::None => Ok(None),
            Self::Some(raw) => Some(AuxData::try_new(raw_to_string(raw))).transpose(),
        }
    }

    pub unsafe fn from_aux_data(aux_data: Option<AuxData>) -> Self {
        if let Some(aux_data) = aux_data {
            let cstr = CString::new(aux_data.0.to_string()).unwrap();
            StableAuxData::Some(cstr.into_raw())
        } else {
            StableAuxData::None
        }
    }
}

impl StableProcMacroResult {
    /// Convert to native Rust representation.
    ///
    /// # Safety
    pub unsafe fn into_proc_macro_result(self) -> ProcMacroResult {
        match self {
            Self::Leave => ProcMacroResult::Leave,
            Self::Remove => ProcMacroResult::Remove,
            Self::Replace {
                token_stream,
                aux_data,
            } => ProcMacroResult::Replace {
                token_stream: token_stream.into_token_stream(),
                aux_data: aux_data.into_aux_data().unwrap(),
            },
        }
    }

    /// Convert to FFI-safe representation.
    ///
    /// # Safety
    pub unsafe fn from_proc_macro_result(result: ProcMacroResult) -> Self {
        match result {
            ProcMacroResult::Leave => StableProcMacroResult::Leave,
            ProcMacroResult::Remove => StableProcMacroResult::Remove,
            ProcMacroResult::Replace {
                token_stream,
                aux_data,
            } => StableProcMacroResult::Replace {
                token_stream: StableTokenStream::from_token_stream(token_stream),
                aux_data: StableAuxData::from_aux_data(aux_data),
            },
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
