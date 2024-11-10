use crate::CONTEXT;
use bumpalo::Bump;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

/// An abstract stream of Cairo tokens.
///
/// This is both input and part of an output of a procedural macro.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "de::TokenStream"))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TokenStream {
    pub tokens: Vec<TokenTree>,
    pub metadata: TokenStreamMetadata,
}

/// This module implements deserialization of the token stream, for the serde feature.
/// This is intermediate representation is needed, as real [`Token`] only contains a reference to the
/// represented string, which needs to be allocated outside the [`Token`] struct.
/// Here we allocate each token to an owned String with SerDe and then copy it's content into context.
#[cfg(feature = "serde")]
#[doc(hidden)]
mod de {
    use crate::{AllocationContext, TextSpan, TokenStreamMetadata};
    use std::fmt::{Display, Formatter};

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct TokenStream {
        pub tokens: Vec<TokenTree>,
        pub metadata: TokenStreamMetadata,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub enum TokenTree {
        Ident(Token),
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Token {
        pub content: String,
        pub span: TextSpan,
    }

    pub struct Error {}

    impl Display for Error {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_str("TokenStream deserialization error")
        }
    }

    impl TryFrom<TokenStream> for crate::TokenStream {
        type Error = Error;

        fn try_from(value: TokenStream) -> Result<Self, Self::Error> {
            let ctx = AllocationContext::default();
            let tokens = value
                .tokens
                .into_iter()
                .map(|token| match token {
                    TokenTree::Ident(token) => {
                        let content = ctx.intern(token.content.as_str());
                        let token = crate::Token {
                            content,
                            span: token.span,
                        };
                        crate::TokenTree::Ident(token)
                    }
                })
                .collect::<Vec<_>>();
            Ok(Self {
                tokens,
                metadata: value.metadata,
            })
        }
    }
}

/// A single token or a delimited sequence of token trees.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenTree {
    Ident(Token),
}

impl Default for TokenTree {
    fn default() -> Self {
        Self::Ident(Default::default())
    }
}

impl TokenTree {
    /// Get the size hint for the [`TokenTree`].
    /// This can be used to estimate size of a buffer needed for allocating this [`TokenTree`].
    pub(crate) fn size_hint(&self) -> usize {
        match self {
            Self::Ident(token) => token.size_hint(),
        }
    }
}

/// A range of text offsets that form a span (like text selection).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}

/// A single Cairo token.
///
/// The most atomic item of Cairo code representation.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    pub content: InternedStr,
    pub span: TextSpan,
}

impl Token {
    /// Get the size hint for the [`Token`].
    /// This can be used to estimate size of a buffer needed for allocating this [`Token`].
    pub(crate) fn size_hint(&self) -> usize {
        self.content.deref().len()
    }
}

/// A wrapper over a string pointer.
/// This contains a pointer to a string allocated in a bump allocator
/// and a guard which keeps the buffer alive.
/// This way we do not need to allocate a new string,
/// but also do not need to worry about the lifetime of the string.
#[derive(Debug, Clone)]
pub struct InternedStr {
    ptr: *const str,
    // Holding a rc to the underlying buffer, so that ptr will always point to valid memory.
    _bump: Rc<BumpWrap>,
}

impl InternedStr {
    #[allow(private_interfaces)]
    #[doc(hidden)]
    pub(crate) fn new_in(s: &str, bump: Rc<BumpWrap>) -> Self {
        let allocated = bump.0.alloc_str(s);
        let ptr = allocated as *const str;
        Self { ptr, _bump: bump }
    }
}

impl Default for InternedStr {
    fn default() -> Self {
        Self {
            ptr: "" as *const str,
            _bump: Rc::default(),
        }
    }
}

impl AsRef<str> for InternedStr {
    fn as_ref(&self) -> &str {
        self.deref()
    }
}

impl Deref for InternedStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for InternedStr {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_ref())
    }
}

impl PartialEq for InternedStr {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}

impl Eq for InternedStr {}

impl Hash for InternedStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

/// This wrapper de-allocates the underlying buffer on drop.
#[derive(Debug, Default)]
struct BumpWrap(pub Bump);

impl Drop for BumpWrap {
    fn drop(&mut self) {
        self.0.reset();
    }
}

/// A context for allocating Cairo tokens.
/// This wrapper contains a bump allocator, which is used to allocate strings for tokens.
#[derive(Clone)]
pub struct AllocationContext {
    bump: Rc<BumpWrap>,
}

impl AllocationContext {
    /// Allocate a new context with pre-determined buffer size.
    pub fn with_capacity(size_hint: usize) -> Self {
        Self {
            bump: Rc::new(BumpWrap(Bump::with_capacity(size_hint))),
        }
    }

    /// Allocate a string in the context.
    /// This returned a string pointer, guarded by reference counter to the buffer.
    /// The buffer will be deallocated when the last reference to the buffer is dropped.
    /// No special handling or lifetimes are needed for the string.
    pub(crate) fn intern(&self, value: &str) -> InternedStr {
        InternedStr::new_in(value, self.bump.clone())
    }
}

impl Default for AllocationContext {
    fn default() -> Self {
        Self {
            bump: Rc::new(BumpWrap(Bump::new())),
        }
    }
}

/// Metadata of [`TokenStream`].
///
/// This struct describes the origin of the [`TokenStream`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TokenStreamMetadata {
    /// The path to the file from which the [`TokenStream`] has been created.
    pub original_file_path: Option<String>,
    /// ID of the file from which the [`TokenStream`] has been created.
    ///
    /// It is guaranteed, that the `file_id` will be unique for each file.
    pub file_id: Option<String>,
    /// Cairo edition defined for the token stream.
    pub edition: Option<String>,
}

impl TokenStream {
    #[doc(hidden)]
    pub fn new(tokens: Vec<TokenTree>) -> Self {
        Self {
            tokens,
            metadata: TokenStreamMetadata::default(),
        }
    }

    #[doc(hidden)]
    pub fn empty() -> Self {
        Self::new(Vec::default())
    }

    #[doc(hidden)]
    pub fn with_metadata(mut self, metadata: TokenStreamMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Get `[TokenStreamMetadata`] associated with this [`TokenStream`].
    ///
    /// The metadata struct can be used to describe the [`TokenStream`] origin.
    pub fn metadata(&self) -> &TokenStreamMetadata {
        &self.metadata
    }

    /// Check if the [`TokenStream`] is empty.
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}

impl Display for TokenStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for token in &self.tokens {
            match token {
                TokenTree::Ident(token) => {
                    write!(f, "{}", token.content.as_ref())?;
                }
            }
        }
        Ok(())
    }
}

impl TokenStreamMetadata {
    #[doc(hidden)]
    pub fn new(file_path: impl ToString, file_id: impl ToString, edition: impl ToString) -> Self {
        Self {
            original_file_path: Some(file_path.to_string()),
            file_id: Some(file_id.to_string()),
            edition: Some(edition.to_string()),
        }
    }
}

impl TokenTree {
    /// Create a new [`TokenTree`] from an identifier [`Token`].
    pub fn from_ident(token: Token) -> Self {
        Self::Ident(token)
    }
}

impl TextSpan {
    /// Create a new [`TextSpan`].
    pub fn new(start: usize, end: usize) -> TextSpan {
        TextSpan { start, end }
    }
}

impl Token {
    /// Create [`Token`] in thread-local context.
    pub fn new(content: impl AsRef<str>, span: TextSpan) -> Self {
        CONTEXT.with(|ctx| {
            let ctx_borrow = ctx.borrow();
            let ctx: &AllocationContext = ctx_borrow.deref();
            Self::new_in(content, span, ctx)
        })
    }

    /// Create [`Token`] in specified context.
    pub fn new_in(content: impl AsRef<str>, span: TextSpan, ctx: &AllocationContext) -> Self {
        let content = ctx.intern(content.as_ref());
        Self { content, span }
    }
}
