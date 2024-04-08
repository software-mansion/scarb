use crate::ffi::{StableOption, StableSlice};
use std::ffi::CStr;
use std::num::NonZeroU8;
use std::os::raw::c_char;
use std::ptr::NonNull;

pub mod ffi;

#[repr(C)]
#[derive(Debug)]
pub struct StableExpansion {
    pub name: *mut c_char,
    pub kind: StableExpansionKind,
}

pub type StableExpansionKind = NonZeroU8;

pub type StableExpansionsList = StableSlice<StableExpansion>;

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
pub struct StableProcMacroResult {
    pub token_stream: StableTokenStream,
    pub diagnostics: StableSlice<StableDiagnostic>,
    pub aux_data: StableAuxData,
    pub full_path_markers: StableSlice<*mut c_char>,
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

#[repr(C)]
pub struct StablePostProcessContext {
    pub aux_data: StableSlice<StableAuxData>,
    pub full_path_markers: StableSlice<StableFullPathMarker>,
}

#[repr(C)]
pub struct StableFullPathMarker {
    pub key: *mut c_char,
    pub full_path: *mut c_char,
}
