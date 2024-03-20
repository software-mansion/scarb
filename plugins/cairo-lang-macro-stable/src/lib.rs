use crate::ffi::StableSlice;
use std::ffi::CStr;
use std::num::NonZeroU8;
use std::os::raw::c_char;
use std::ptr::NonNull;

pub mod ffi;

/// An option.
///
/// This struct implements FFI-safe stable ABI.
#[repr(C)]
#[derive(Debug)]
pub enum StableOption<T> {
    None,
    Some(T),
}

/// Token stream.
///
/// This struct implements FFI-safe stable ABI.
#[repr(C)]
#[derive(Debug)]
pub struct StableTokenStream {
    pub value: *mut c_char,
    pub metadata: StableTokenStreamMetadata,
}

/// Token stream metadata.
///
/// This struct implements FFI-safe stable ABI.
#[repr(C)]
#[derive(Debug)]
pub struct StableTokenStreamMetadata {
    pub original_file_path: Option<NonNull<c_char>>,
    pub file_id: Option<NonNull<c_char>>,
}

/// Auxiliary data returned by the procedural macro.
///
/// This struct implements FFI-safe stable ABI.
pub type StableAuxData = StableOption<StableSlice<u8>>;

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
    /// Convert to String.
    ///
    /// # Safety
    pub unsafe fn to_string(&self) -> String {
        // Note that this does not deallocate the c-string.
        // The memory must still be freed with `CString::from_raw`.
        CStr::from_ptr(self.value).to_string_lossy().to_string()
    }
}
