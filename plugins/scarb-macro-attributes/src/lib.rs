use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Inline macro helper.
///
/// This macro hides the conversion to stable ABI structs from the user.
///
/// # Safety
/// Note that token stream deserialization may fail.
#[proc_macro_attribute]
pub fn attribute_macro(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item: ItemFn = parse_macro_input!(input as ItemFn);
    let item_name = &item.sig.ident;
    let expanded = quote! {
        #item

        #[no_mangle]
        pub unsafe extern "C" fn expand(token_stream: scarb_macro_interface::stable_abi::StableTokenStream) -> scarb_macro_interface::stable_abi::StableProcMacroResult {
            let token_stream = token_stream.into_token_stream();
            let result = #item_name(token_stream);
            scarb_macro_interface::stable_abi::StableProcMacroResult::from_proc_macro_result(result)
        }
    };
    TokenStream::from(expanded)
}
