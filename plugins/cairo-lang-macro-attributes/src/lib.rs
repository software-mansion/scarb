use proc_macro::TokenStream;
use quote::quote;
use scarb_stable_hash::short_hash;
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
    let original_item_name = item.sig.ident.to_string();
    let item = hide_name(item);
    let item_name = &item.sig.ident;
    let expanded = quote! {
        #item

        #[::cairo_lang_macro::linkme::distributed_slice(::cairo_lang_macro::MACRO_DEFINITIONS_SLICE)]
        #[linkme(crate = ::cairo_lang_macro::linkme)]
        static MACRO_DEFINITIONS_SLICE_DESERIALIZE: ::cairo_lang_macro::ExpansionDefinition =
            ::cairo_lang_macro::ExpansionDefinition{
                name: #original_item_name,
                kind: ::cairo_lang_macro::ExpansionKind::Attr,
                fun: #item_name,
            };
    };
    TokenStream::from(expanded)
}

/// This macro can be used to construct the auxiliary data collection callback.
///
/// The procedural macro can emit additional auxiliary data alongside the generated [`TokenStream`]
/// during the code expansion. This data can be used to collect additional information from the
/// source code of a project that is being compiled during the macro execution.
/// For instance, you can create a procedural macro that collects some information stored by
/// the Cairo programmer as attributes in the project source code.
///
/// This should be used to implement a collection callback for the auxiliary data.
/// This callback will be called after the source code compilation (and thus after all the procedural
/// macro executions). All auxiliary data emitted by the procedural macro during source code compilation
/// will be passed to the callback as an argument.
///
/// The callback can be used to process or persist the data collected during the compilation.
///
/// This macro hides the conversion to stable ABI structs from the user.
///
/// # Safety
#[proc_macro_attribute]
pub fn aux_data_collection_callback(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item: ItemFn = parse_macro_input!(input as ItemFn);
    let item = hide_name(item);
    let item_name = &item.sig.ident;
    let expanded = quote! {
        #item

        #[::cairo_lang_macro::linkme::distributed_slice(::cairo_lang_macro::AUX_DATA_CALLBACKS)]
        #[linkme(crate = ::cairo_lang_macro::linkme)]
        static AUX_DATA_CALLBACK_DESERIALIZE: fn(Vec<AuxData>) = #item_name;
    };
    TokenStream::from(expanded)
}

/// Rename item to hide it from the macro source code.
fn hide_name(mut item: ItemFn) -> ItemFn {
    let id = short_hash(item.sig.ident.to_string());
    let item_name = format!("{}_{}", item.sig.ident, id);
    item.sig.ident = syn::Ident::new(item_name.as_str(), item.sig.ident.span());
    item
}
