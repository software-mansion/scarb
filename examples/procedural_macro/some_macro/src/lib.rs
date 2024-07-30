use cairo_lang_macro::{attribute_macro, ProcMacroResult, TokenStream};

#[attribute_macro]
pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
    let token_stream = TokenStream::new(token_stream.to_string().replace("12", "34"));
    ProcMacroResult::new(token_stream)
}
