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
        pub unsafe extern "C" fn expand(token_stream: cairo_lang_macro_stable::StableTokenStream) -> cairo_lang_macro_stable::StableProcMacroResult {
            let token_stream = TokenStream::from_stable(token_stream);
            let result = #item_name(token_stream);
            result.into_stable()
        }
    };
    TokenStream::from(expanded)
}
