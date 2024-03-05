use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Token stream.
///
/// This struct implements FFI-safe stable ABI.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct StableTokenStream(*mut c_char);

#[repr(C)]
#[derive(Debug, Clone)]
pub enum StableAuxData {
    None,
    Some(*mut c_char),
}

/// Procedural macro result.
///
/// This struct implements FFI-safe stable ABI.
#[repr(C)]
#[derive(Debug, Clone)]
pub enum StableProcMacroResult {
    /// Plugin has not taken any action.
    Leave,
    /// Plugin generated [`StableTokenStream`] replacement.
    Replace {
        token_stream: StableTokenStream,
        aux_data: StableAuxData,
    },
    /// Plugin ordered item removal.
    Remove,
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
