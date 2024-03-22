use proc_macro::TokenStream;
use quote::quote;
use scarb_stable_hash::short_hash;
use syn::{parse_macro_input, ItemFn};

/// Constructs the attribute macro implementation.
///
/// This macro hides the conversion to stable ABI structs from the user.
///
/// Note, that this macro can be used multiple times, to define multiple independent attribute macros.
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

/// Constructs the post-processing callback.
///
/// This callback will be called after the source code compilation (and thus after all the procedural
/// macro expansion calls).
/// The post-processing callback is the only function defined by the procedural macro that is
/// allowed to have side effects.
///
/// This macro will be called with a list of all auxiliary data emitted by the macro during code expansion.
///
/// This data can be used to collect additional information from the source code of a project
/// that is being compiled during the macro execution.
/// For instance, you can create a procedural macro that collects some information stored by
/// the Cairo programmer as attributes in the project source code.
/// This callback will be called after the source code compilation (and thus after all the procedural
/// macro executions). All auxiliary data emitted by the procedural macro during source code compilation
/// will be passed to the callback as an argument.
///
/// This macro hides the conversion to stable ABI structs from the user.
///
/// If multiple callbacks are defined within the macro, all the implementations will be executed.
/// No guarantees can be made regarding the order of execution.
#[proc_macro_attribute]
pub fn post_process(_args: TokenStream, input: TokenStream) -> TokenStream {
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
