use crate::ffi::StableSlice;
use std::ffi::{CStr, CString};
use std::num::NonZeroU8;
use std::os::raw::c_char;

pub mod ffi;

/// Token stream.
///
/// This struct implements FFI-safe stable ABI.
#[repr(C)]
#[derive(Debug)]
pub struct StableTokenStream(*mut c_char);

#[repr(C)]
#[derive(Debug)]
pub enum StableAuxData {
    None,
    Some(StableSlice<u8>),
}

/// Diagnostic returned by the procedural macro.
///
/// This struct implements FFI-safe stable ABI.
#[repr(C)]
#[derive(Debug)]
pub struct StableDiagnostic {
    pub message: *mut c_char,
    pub severity: StableSeverity,
}

/// The severity of a diagnostic.
///
/// This struct implements FFI-safe stable ABI.
pub type StableSeverity = NonZeroU8;

/// Procedural macro result.
///
/// This struct implements FFI-safe stable ABI.
#[repr(C)]
#[derive(Debug)]
pub enum StableProcMacroResult {
    /// Plugin has not taken any action.
    Leave {
        diagnostics: StableSlice<StableDiagnostic>,
    },
    /// Plugin generated [`StableTokenStream`] replacement.
    Replace {
        diagnostics: StableSlice<StableDiagnostic>,
        token_stream: StableTokenStream,
        aux_data: StableAuxData,
    },
    /// Plugin ordered item removal.
    Remove {
        diagnostics: StableSlice<StableDiagnostic>,
    },
}

#[repr(C)]
pub struct StableResultWrapper {
    pub input: StableTokenStream,
    pub output: StableProcMacroResult,
}

impl StableTokenStream {
    pub fn new(s: *mut c_char) -> Self {
        Self(s)
    }

    /// Convert to String.
    ///
    /// # Safety
    pub unsafe fn to_string(&self) -> String {
        // Note that this does not deallocate the c-string.
        // The memory must still be freed with `CString::from_raw`.
        CStr::from_ptr(self.0).to_string_lossy().to_string()
    }

    pub fn into_owned_string(self) -> String {
        unsafe { raw_to_string(self.0) }
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
