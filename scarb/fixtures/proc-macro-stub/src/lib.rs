use scarb_macro_attributes::attribute_macro;
use scarb_macro_interface::{ProcMacroResult, TokenStream};

/// Procedural macro stub.
#[attribute_macro]
pub fn some_macro(token_stream: TokenStream) -> ProcMacroResult {
    let _code = token_stream.to_string();
    ProcMacroResult::Leave
}
