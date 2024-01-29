use crate::{ProcMacroResult, TokenStream};
use std::ffi::CString;
use std::os::raw::c_char;

/// Token stream.
///
/// This struct implements FFI-safe stable ABI.
#[repr(C)]
#[derive(Debug)]
pub struct StableTokenStream(pub *mut c_char);

/// Procedural macro result.
///
/// This struct implements FFI-safe stable ABI.
#[repr(C)]
#[derive(Debug)]
pub enum StableProcMacroResult {
    /// Plugin has not taken any action.
    Leave,
    /// Plugin generated [`TokenStream`] replacement.
    Replace(StableTokenStream),
    /// Plugin ordered item removal.
    Remove,
}

impl StableTokenStream {
    /// Convert to String.
    ///
    /// # Safety
    pub unsafe fn to_string(&self) -> String {
        if self.0.is_null() {
            String::default()
        } else {
            let cstr = CString::from_raw(self.0);
            cstr.to_string_lossy().to_string()
        }
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

impl StableProcMacroResult {
    /// Convert to native Rust representation.
    ///
    /// # Safety
    pub unsafe fn into_proc_macro_result(self) -> ProcMacroResult {
        match self {
            Self::Leave => ProcMacroResult::Leave,
            Self::Remove => ProcMacroResult::Remove,
            Self::Replace(token_stream) => {
                ProcMacroResult::Replace(token_stream.into_token_stream())
            }
        }
    }

    /// Convert to FFI-safe representation.
    ///
    /// # Safety
    pub unsafe fn from_proc_macro_result(result: ProcMacroResult) -> Self {
        match result {
            ProcMacroResult::Leave => StableProcMacroResult::Leave,
            ProcMacroResult::Remove => StableProcMacroResult::Remove,
            ProcMacroResult::Replace(token_stream) => {
                StableProcMacroResult::Replace(StableTokenStream::from_token_stream(token_stream))
            }
        }
    }
}
