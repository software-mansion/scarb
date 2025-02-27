use cairo_lang_macro_v2::{attribute_macro, ProcMacroResult, TokenStream};

#[attribute_macro(unsupported_key = "some::path")]
fn t1(_a: TokenStream, _b: TokenStream) -> ProcMacroResult {
    ProcMacroResult::new(TokenStream::empty())
}

fn main() {}
