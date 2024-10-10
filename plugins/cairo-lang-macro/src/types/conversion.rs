use crate::{
    AuxData, Diagnostic, ExpansionDefinition, FullPathMarker, PostProcessContext, ProcMacroResult,
    Severity, TokenStream, TokenStreamMetadata,
};
use cairo_lang_macro_stable::ffi::StableSlice;
use cairo_lang_macro_stable::{
    StableAuxData, StableDiagnostic, StableExpansion, StableFullPathMarker,
    StablePostProcessContext, StableProcMacroResult, StableSeverity, StableTokenStream,
    StableTokenStreamMetadata,
};
use std::ffi::{c_char, CStr, CString};
use std::num::NonZeroU8;
use std::ptr::NonNull;
use std::slice;

impl ProcMacroResult {
    /// Convert to FFI-safe representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub fn into_stable(self) -> StableProcMacroResult {
        let diagnostics = self
            .diagnostics
            .into_iter()
            .map(|d| d.into_stable())
            .collect::<Vec<_>>();
        let full_path_markers = self
            .full_path_markers
            .into_iter()
            .map(|m| CString::new(m).unwrap().into_raw())
            .collect::<Vec<_>>();
        StableProcMacroResult {
            token_stream: self.token_stream.into_stable(),
            aux_data: AuxData::maybe_into_stable(self.aux_data),
            diagnostics: StableSlice::new(diagnostics),
            full_path_markers: StableSlice::new(full_path_markers),
        }
    }

    /// Convert to native Rust representation, without taking the ownership of the string.
    ///
    /// Note that you still need to free the memory by calling `from_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(result: &StableProcMacroResult) -> Self {
        let (ptr, n) = result.diagnostics.raw_parts();
        let diagnostics = slice::from_raw_parts(ptr, n)
            .iter()
            .map(|d| Diagnostic::from_stable(d))
            .collect::<Vec<_>>();
        let (ptr, n) = result.full_path_markers.raw_parts();
        let full_path_markers = slice::from_raw_parts(ptr, n)
            .iter()
            .map(|m| from_raw_cstr(*m))
            .collect::<Vec<_>>();
        ProcMacroResult {
            token_stream: TokenStream::from_stable(&result.token_stream),
            diagnostics,
            full_path_markers,
            aux_data: AuxData::from_stable(&result.aux_data),
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
        let diagnostics = result.diagnostics.into_owned();
        let diagnostics = diagnostics
            .into_iter()
            .map(|d| Diagnostic::from_owned_stable(d))
            .collect::<Vec<_>>();
        let full_path_markers = result
            .full_path_markers
            .into_owned()
            .iter()
            .map(|m| from_raw_cstring(*m))
            .collect::<Vec<_>>();
        ProcMacroResult {
            token_stream: TokenStream::from_owned_stable(result.token_stream),
            aux_data: AuxData::from_owned_stable(result.aux_data),
            diagnostics,
            full_path_markers,
        }
    }
}

impl TokenStream {
    /// Convert to FFI-safe representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub fn into_stable(self) -> StableTokenStream {
        let cstr = CString::new(serde_json::to_string(&self.tokens).unwrap()).unwrap();
        StableTokenStream {
            value: cstr.into_raw(),
            metadata: self.metadata.into_stable(),
        }
    }

    /// Convert to native Rust representation, without taking the ownership of the string.
    ///
    /// Note that you still need to free the memory by calling `from_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(token_stream: &StableTokenStream) -> Self {
        Self {
            tokens: serde_json::from_str(&from_raw_cstr(token_stream.value)).unwrap(),
            metadata: TokenStreamMetadata::from_stable(&token_stream.metadata),
        }
    }

    /// Convert to native Rust representation, with taking the ownership of the string.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_owned_stable(token_stream: StableTokenStream) -> Self {
        Self {
            tokens: serde_json::from_str(&from_raw_cstr(token_stream.value)).unwrap(),
            metadata: TokenStreamMetadata::from_owned_stable(token_stream.metadata),
        }
    }
}

impl TokenStreamMetadata {
    /// Convert to FFI-safe representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub fn into_stable(self) -> StableTokenStreamMetadata {
        let original_file_path = self
            .original_file_path
            .and_then(|path| NonNull::new(CString::new(path).unwrap().into_raw()));
        let file_id = self
            .file_id
            .and_then(|path| NonNull::new(CString::new(path).unwrap().into_raw()));
        StableTokenStreamMetadata {
            original_file_path,
            file_id,
        }
    }

    /// Convert to native Rust representation, without taking the ownership of the string.
    ///
    /// Note that you still need to free the memory by calling `from_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(metadata: &StableTokenStreamMetadata) -> Self {
        let original_file_path = metadata
            .original_file_path
            .map(|raw| from_raw_cstr(raw.as_ptr()));
        let file_id = metadata.file_id.map(|raw| from_raw_cstr(raw.as_ptr()));
        Self {
            original_file_path,
            file_id,
        }
    }

    /// Convert to native Rust representation, with taking the ownership of the string.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_owned_stable(metadata: StableTokenStreamMetadata) -> Self {
        let original_file_path = metadata
            .original_file_path
            .map(|raw| from_raw_cstring(raw.as_ptr()));
        let file_id = metadata.file_id.map(|raw| from_raw_cstring(raw.as_ptr()));
        Self {
            original_file_path,
            file_id,
        }
    }
}

impl AuxData {
    /// Convert to FFI-safe representation.
    ///
    /// # Safety
    #[doc(hidden)]
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
        let value: Vec<u8> = self.into();
        StableAuxData::Some(StableSlice::new(value))
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
            StableAuxData::Some(raw) => {
                let (ptr, n) = raw.raw_parts();
                let value = slice::from_raw_parts(ptr, n);
                Some(value.into())
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
    pub unsafe fn from_owned_stable(aux_data: StableAuxData) -> Option<Self> {
        match aux_data {
            StableAuxData::None => None,
            StableAuxData::Some(raw) => Some(Self::new(raw.into_owned())),
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

impl ExpansionDefinition {
    // Convert to FFI-safe representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub fn into_stable(self) -> StableExpansion {
        StableExpansion {
            name: CString::new(self.name).unwrap().into_raw(),
            kind: self.kind.into_stable(),
        }
    }

    /// Take the ownership of the string.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn free_owned(expansion: StableExpansion) {
        let _ = from_raw_cstring(expansion.name);
    }
}

impl FullPathMarker {
    // Convert to FFI-safe representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub fn into_stable(self) -> StableFullPathMarker {
        StableFullPathMarker {
            key: CString::new(self.key).unwrap().into_raw(),
            full_path: CString::new(self.full_path).unwrap().into_raw(),
        }
    }

    /// Convert to native Rust representation, without taking the ownership of the string.
    ///
    /// Note that you still need to free the memory by calling `from_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(marker: &StableFullPathMarker) -> Self {
        Self {
            key: from_raw_cstr(marker.key),
            full_path: from_raw_cstr(marker.full_path),
        }
    }

    /// Convert to native Rust representation, with taking the ownership of the string.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_owned_stable(marker: StableFullPathMarker) -> Self {
        Self {
            key: from_raw_cstring(marker.key),
            full_path: from_raw_cstring(marker.full_path),
        }
    }
}

impl PostProcessContext {
    // Convert to FFI-safe representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub fn into_stable(self) -> StablePostProcessContext {
        let aux_data = self
            .aux_data
            .into_iter()
            .map(|a| a.into_stable())
            .collect::<Vec<_>>();

        let full_path_markers = self
            .full_path_markers
            .into_iter()
            .map(|m| m.into_stable())
            .collect::<Vec<_>>();

        StablePostProcessContext {
            aux_data: StableSlice::new(aux_data),
            full_path_markers: StableSlice::new(full_path_markers),
        }
    }

    /// Convert to native Rust representation, without taking the ownership of the string.
    ///
    /// Note that you still need to free the memory by calling `from_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(context: &StablePostProcessContext) -> Self {
        let (ptr, n) = context.aux_data.raw_parts();
        let aux_data = slice::from_raw_parts(ptr, n)
            .iter()
            .filter_map(|a| AuxData::from_stable(a))
            .collect::<Vec<_>>();
        let (ptr, n) = context.full_path_markers.raw_parts();
        let full_path_markers = slice::from_raw_parts(ptr, n)
            .iter()
            .map(|m| FullPathMarker::from_stable(m))
            .collect::<Vec<_>>();
        Self {
            aux_data,
            full_path_markers,
        }
    }

    /// Convert to native Rust representation, with taking the ownership of the string.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_owned_stable(diagnostic: StablePostProcessContext) -> Self {
        let aux_data = diagnostic
            .aux_data
            .into_owned()
            .into_iter()
            .filter_map(|a| AuxData::from_owned_stable(a))
            .collect::<Vec<_>>();
        let full_path_markers = diagnostic
            .full_path_markers
            .into_owned()
            .into_iter()
            .map(|m| FullPathMarker::from_owned_stable(m))
            .collect::<Vec<_>>();
        Self {
            aux_data,
            full_path_markers,
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
