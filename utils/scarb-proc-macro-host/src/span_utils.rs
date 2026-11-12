use cairo_lang_macro::{TextOffset, TokenStream, TokenTree};

/// Move start and end of each span of each token in the `token_stream` to the left by `offset`.
pub fn move_spans_by_offset(token_stream: TokenStream, offset: TextOffset) -> TokenStream {
    TokenStream::new(
        token_stream
            .into_iter()
            .map(|tree| match tree {
                TokenTree::Ident(mut token) => {
                    token.span.start -= offset;
                    token.span.end -= offset;
                    TokenTree::Ident(token)
                }
            })
            .collect(),
    )
}
