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

#[cfg(feature = "serde")]
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
            f.write_str("deserialization error")
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

/// A range of text offsets that form a span (like text selection).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}

/// A single Cairo token.
///
/// The most atomic item, of Cairo code representation, when passed between macro and host.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    pub content: InternedStr,
    pub span: TextSpan,
}

#[derive(Debug, Clone)]
pub struct InternedStr {
    ptr: *const str,
    // Holding a rc to the underlying buffer, so that ptr will always point to valid memory.
    _bump: Rc<BumpWrap>,
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

#[derive(Debug, Default)]
pub struct BumpWrap(pub Bump);

impl InternedStr {
    pub fn as_bytes(&self) -> &[u8] {
        let ptr: &str = unsafe { &*self.ptr };
        ptr.as_bytes()
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

impl InternedStr {
    pub fn new_in(s: &str, bump: Rc<BumpWrap>) -> Self {
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

pub struct AllocationContext {
    bump: Rc<BumpWrap>,
}

impl AllocationContext {
    pub fn intern(&self, value: &str) -> InternedStr {
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

impl Drop for BumpWrap {
    fn drop(&mut self) {
        // println!("dropped");
        self.0.reset();
    }
}

/// Metadata of [`TokenStream`].
///
/// This struct can be used to describe the origin of the [`TokenStream`].
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
    pub fn from_ident(token: Token) -> Self {
        Self::Ident(token)
    }
}

impl TextSpan {
    pub fn new(start: usize, end: usize) -> TextSpan {
        TextSpan { start, end }
    }
}

impl Token {
    pub fn new(content: impl AsRef<str>, span: TextSpan) -> Self {
        CONTEXT.with(|ctx| {
            let ctx = ctx.get_or_init(|| Rc::new(AllocationContext::default()));
            let ctx = ctx.clone();
            let content = ctx.intern(content.as_ref());
            Self { content, span }
        })
    }
}
