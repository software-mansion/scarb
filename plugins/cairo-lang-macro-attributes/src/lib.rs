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
        pub unsafe extern "C" fn expand(stable_token_stream: cairo_lang_macro_stable::StableTokenStream) -> cairo_lang_macro_stable::StableResultWrapper {
            let token_stream = cairo_lang_macro::TokenStream::from_stable(&stable_token_stream);
            let result = #item_name(token_stream);
            let result: cairo_lang_macro_stable::StableProcMacroResult = result.into_stable();
            cairo_lang_macro_stable::StableResultWrapper {
                input: stable_token_stream,
                output: result,
            }
        }
    };
    TokenStream::from(expanded)
}

/// AuxData callback helper.
///
/// This macro hides the conversion to stable ABI structs from the user.
///
/// # Safety
/// Note that AuxData deserialization may fail.
#[proc_macro_attribute]
pub fn aux_data_callback(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item: ItemFn = parse_macro_input!(input as ItemFn);
    let item_name = &item.sig.ident;
    let expanded = quote! {
        #item

        #[no_mangle]
        pub unsafe extern "C" fn aux_data_callback(stable_aux_data: cairo_lang_macro_stable::ffi::StableSlice<cairo_lang_macro_stable::StableAuxData>) ->  cairo_lang_macro_stable::ffi::StableSlice<cairo_lang_macro_stable::StableAuxData> {
            let (ptr, n) = stable_aux_data.raw_parts();
            let aux_data: &[cairo_lang_macro_stable::StableAuxData] = std::slice::from_raw_parts(ptr, n);
            let aux_data = aux_data.iter().filter_map(|a| cairo_lang_macro::AuxData::from_stable(a)).collect::<Vec<_>>();
            #item_name(aux_data);
            stable_aux_data
        }
    };
    TokenStream::from(expanded)
}
