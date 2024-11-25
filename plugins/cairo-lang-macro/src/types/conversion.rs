use crate::{
    AllocationContext, AuxData, Diagnostic, ExpansionDefinition, FullPathMarker,
    PostProcessContext, ProcMacroResult, Severity, TextSpan, Token, TokenStream,
    TokenStreamMetadata, TokenTree,
};
use cairo_lang_macro_stable::ffi::StableSlice;
use cairo_lang_macro_stable::{
    StableAuxData, StableDiagnostic, StableExpansion, StableFullPathMarker,
    StablePostProcessContext, StableProcMacroResult, StableSeverity, StableTextSpan, StableToken,
    StableTokenStream, StableTokenStreamMetadata, StableTokenTree,
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
    /// Note that you still need to free the memory by calling `free_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(result: &StableProcMacroResult) -> Self {
        let ctx = AllocationContext::with_capacity(result.token_stream.size_hint);
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
            token_stream: TokenStream::from_stable_in(&result.token_stream, &ctx),
            diagnostics,
            full_path_markers,
            aux_data: AuxData::from_stable(&result.aux_data),
        }
    }

    /// Take the ownership of memory under the pointer and drop it.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn free_owned_stable(result: StableProcMacroResult) {
        for diagnostic in result.diagnostics.into_owned() {
            Diagnostic::free_owned_stable(diagnostic);
        }
        for marker in result.full_path_markers.into_owned() {
            free_raw_cstring(marker)
        }
        TokenStream::free_owned_stable(result.token_stream);
        AuxData::free_owned_stable(result.aux_data);
    }
}

impl TextSpan {
    /// Convert to FFI-safe representation.
    #[doc(hidden)]
    pub fn into_stable(self) -> StableTextSpan {
        StableTextSpan {
            start: self.start,
            end: self.end,
        }
    }

    /// Convert to native Rust representation, without taking the ownership.
    #[doc(hidden)]
    pub fn from_stable(span: &StableTextSpan) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }

    #[doc(hidden)]
    pub fn free_owned_stable(span: StableTextSpan) {
        let _ = span;
    }
}

impl Token {
    /// Convert to FFI-safe representation.
    #[doc(hidden)]
    pub fn into_stable(self) -> StableToken {
        let ptr = self.content.as_ptr();
        let len = self.content.len();
        StableToken {
            span: self.span.map(|span| span.into_stable()),
            ptr,
            len,
        }
    }

    /// Convert to native Rust representation, without taking the ownership of the string.
    ///
    /// Note that you still need to free the memory by calling `free_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable_in(token: &StableToken, ctx: &AllocationContext) -> Self {
        let content = slice::from_raw_parts(token.ptr, token.len);
        let content = ctx.intern(std::str::from_utf8_unchecked(content));
        Self {
            content,
            span: token.span.as_ref().map(TextSpan::from_stable),
        }
    }

    /// Take the ownership of memory under the pointer and drop it.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn free_owned_stable(token: StableToken) {
        if let Some(token_span) = token.span {
            TextSpan::free_owned_stable(token_span);
        }
    }
}

impl TokenTree {
    /// Convert to FFI-safe representation.
    #[doc(hidden)]
    pub fn into_stable(self) -> StableTokenTree {
        match self {
            Self::Ident(token) => StableTokenTree::Ident(token.into_stable()),
        }
    }

    /// Convert to native Rust representation, without taking the ownership of the string.
    ///
    /// Note that you still need to free the memory by calling `free_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable_in(token_tree: &StableTokenTree, ctx: &AllocationContext) -> Self {
        match token_tree {
            StableTokenTree::Ident(token) => Self::Ident(Token::from_stable_in(token, ctx)),
        }
    }

    /// Take the ownership of memory under the pointer and drop it.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn free_owned_stable(token_tree: StableTokenTree) {
        match token_tree {
            StableTokenTree::Ident(token) => {
                Token::free_owned_stable(token);
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
        let mut size_hint: usize = 0;
        let tokens = self
            .tokens
            .into_iter()
            .map(|token| {
                size_hint += token.size_hint();
                token.into_stable()
            })
            .collect::<Vec<_>>();
        StableTokenStream {
            tokens: StableSlice::new(tokens),
            metadata: self.metadata.into_stable(),
            size_hint,
        }
    }

    /// Convert to native Rust representation, without taking the ownership of the string.
    ///
    /// Note that you still need to free the memory by calling `free_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable_in(
        token_stream: &StableTokenStream,
        ctx: &AllocationContext,
    ) -> Self {
        let (ptr, n) = token_stream.tokens.raw_parts();
        let tokens = slice::from_raw_parts(ptr, n)
            .iter()
            .map(|token_tree| TokenTree::from_stable_in(token_tree, ctx))
            .collect::<Vec<_>>();
        Self {
            tokens,
            metadata: TokenStreamMetadata::from_stable(&token_stream.metadata),
        }
    }

    /// Take the ownership of memory under the pointer and drop it.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn free_owned_stable(token_stream: StableTokenStream) {
        for token_tree in token_stream.tokens.into_owned() {
            TokenTree::free_owned_stable(token_tree);
        }
        TokenStreamMetadata::free_owned_stable(token_stream.metadata);
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
            .and_then(|value| NonNull::new(CString::new(value).unwrap().into_raw()));
        let file_id = self
            .file_id
            .and_then(|value| NonNull::new(CString::new(value).unwrap().into_raw()));
        let edition = self
            .edition
            .and_then(|value| NonNull::new(CString::new(value).unwrap().into_raw()));
        StableTokenStreamMetadata {
            original_file_path,
            file_id,
            edition,
        }
    }

    /// Convert to native Rust representation, without taking the ownership of the string.
    ///
    /// Note that you still need to free the memory by calling `free_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(metadata: &StableTokenStreamMetadata) -> Self {
        let original_file_path = metadata
            .original_file_path
            .map(|raw| from_raw_cstr(raw.as_ptr()));
        let file_id = metadata.file_id.map(|raw| from_raw_cstr(raw.as_ptr()));
        let edition = metadata.edition.map(|raw| from_raw_cstr(raw.as_ptr()));
        Self {
            original_file_path,
            file_id,
            edition,
        }
    }

    /// Take the ownership of memory under the pointer and drop it.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn free_owned_stable(metadata: StableTokenStreamMetadata) {
        if let Some(raw) = metadata.original_file_path {
            free_raw_cstring(raw.as_ptr());
        }
        if let Some(raw) = metadata.file_id {
            free_raw_cstring(raw.as_ptr());
        }
        if let Some(raw) = metadata.edition {
            free_raw_cstring(raw.as_ptr());
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
    /// Note that you still need to free the memory by calling `free_owned_stable`.
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

    /// Take the ownership of memory under the pointer and drop it.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn free_owned_stable(aux_data: StableAuxData) {
        match aux_data {
            StableAuxData::None => {}
            StableAuxData::Some(raw) => {
                let _ = raw.into_owned();
            }
        };
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
    /// Note that you still need to free the memory by calling `free_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(diagnostic: &StableDiagnostic) -> Self {
        Self {
            message: from_raw_cstr(diagnostic.message),
            severity: Severity::from_stable(&diagnostic.severity),
        }
    }

    /// Take the ownership of memory under the pointer and drop it.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn free_owned_stable(diagnostic: StableDiagnostic) {
        free_raw_cstring(diagnostic.message);
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

    /// Take the ownership of memory under the pointer and drop it.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn free_owned(expansion: StableExpansion) {
        free_raw_cstring(expansion.name);
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
    /// Note that you still need to free the memory by calling `free_owned_stable`.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(marker: &StableFullPathMarker) -> Self {
        Self {
            key: from_raw_cstr(marker.key),
            full_path: from_raw_cstr(marker.full_path),
        }
    }

    /// Take the ownership of memory under the pointer and drop it.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn free_owned_stable(marker: StableFullPathMarker) {
        free_raw_cstring(marker.key);
        free_raw_cstring(marker.full_path);
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
    /// Note that you still need to free the memory by calling `free_owned_stable`.
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

    /// Take the ownership of memory under the pointer and drop it.
    ///
    /// Useful when you need to free the allocated memory.
    /// Only use on the same side of FFI-barrier, where the memory has been allocated.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn free_owned_stable(diagnostic: StablePostProcessContext) {
        for aux_data in diagnostic.aux_data.into_owned() {
            AuxData::free_owned_stable(aux_data)
        }
        for marker in diagnostic.full_path_markers.into_owned() {
            FullPathMarker::free_owned_stable(marker);
        }
    }
}

// Create a c-string from a raw pointer to a c_char, and drop it immediately.
// Note that this will free the underlying memory.
unsafe fn free_raw_cstring(raw: *mut c_char) {
    if !raw.is_null() {
        let _ = CString::from_raw(raw);
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
