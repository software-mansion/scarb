use cairo_lang_macro_v2::{inline_macro, ProcMacroResult, TokenStream, MACRO_DEFINITIONS_SLICE};

#[inline_macro(parent = "parent")]
fn t1(_a: TokenStream) -> ProcMacroResult {
    ProcMacroResult::new(TokenStream::empty())
}

fn main() {}
