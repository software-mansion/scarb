use cairo_lang_macro::{attribute_macro, ProcMacroResult, TokenStream};

#[attribute_macro(parent = "a-b")]
fn t1(_a: TokenStream, _b: TokenStream) -> ProcMacroResult {
    ProcMacroResult::new(TokenStream::empty())
}

fn main() {}
