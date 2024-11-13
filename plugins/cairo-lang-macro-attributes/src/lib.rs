use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use scarb_stable_hash::short_hash;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Expr, ItemFn, LitStr, Meta};

/// Constructs the attribute macro implementation.
///
/// This macro hides the conversion to stable ABI structs from the user.
///
/// Note, that this macro can be used multiple times, to define multiple independent attribute macros.
#[proc_macro_attribute]
pub fn attribute_macro(_args: TokenStream, input: TokenStream) -> TokenStream {
    macro_helper(
        input,
        quote!(::cairo_lang_macro::ExpansionKind::Attr),
        quote!(::cairo_lang_macro::ExpansionFunc::Attr),
    )
}

/// Constructs the inline macro implementation.
///
/// This macro hides the conversion to stable ABI structs from the user.
///
/// Note, that this macro can be used multiple times, to define multiple independent attribute macros.
#[proc_macro_attribute]
pub fn inline_macro(_args: TokenStream, input: TokenStream) -> TokenStream {
    macro_helper(
        input,
        quote!(::cairo_lang_macro::ExpansionKind::Inline),
        quote!(::cairo_lang_macro::ExpansionFunc::Other),
    )
}

/// Constructs the derive macro implementation.
///
/// This macro hides the conversion to stable ABI structs from the user.
///
/// Note, that this macro can be used multiple times, to define multiple independent attribute macros.
#[proc_macro_attribute]
pub fn derive_macro(_args: TokenStream, input: TokenStream) -> TokenStream {
    macro_helper(
        input,
        quote!(::cairo_lang_macro::ExpansionKind::Derive),
        quote!(::cairo_lang_macro::ExpansionFunc::Other),
    )
}

fn macro_helper(input: TokenStream, kind: impl ToTokens, func: impl ToTokens) -> TokenStream {
    let item: ItemFn = parse_macro_input!(input as ItemFn);

    let original_item_name = item.sig.ident.to_string();
    let doc = item
        .attrs
        .iter()
        .filter_map(|attr| match &attr.meta {
            Meta::NameValue(meta) => meta.path.is_ident("doc").then(|| match &meta.value {
                Expr::Lit(lit) => match &lit.lit {
                    syn::Lit::Str(lit) => Some(lit.value().trim().to_string()),
                    _ => None,
                },
                _ => None,
            }),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
    let item = hide_name(item);
    let item_name = &item.sig.ident;

    let callback_link = format!(
        "EXPANSIONS_DESERIALIZE_{}",
        item_name.to_string().to_uppercase()
    );

    let callback_link = syn::Ident::new(callback_link.as_str(), item.span());

    let expanded = quote! {
        #item

        #[::cairo_lang_macro::linkme::distributed_slice(::cairo_lang_macro::MACRO_DEFINITIONS_SLICE)]
        #[linkme(crate = ::cairo_lang_macro::linkme)]
        static #callback_link: ::cairo_lang_macro::ExpansionDefinition =
            ::cairo_lang_macro::ExpansionDefinition{
                name: #original_item_name,
                doc: #doc,
                kind: #kind,
                fun: #func(#item_name),
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

    let callback_link = format!(
        "POST_PROCESS_DESERIALIZE_{}",
        item_name.to_string().to_uppercase()
    );
    let callback_link = syn::Ident::new(callback_link.as_str(), item.span());

    let expanded = quote! {
        #item

        #[::cairo_lang_macro::linkme::distributed_slice(::cairo_lang_macro::AUX_DATA_CALLBACKS)]
        #[linkme(crate = ::cairo_lang_macro::linkme)]
        static #callback_link: fn(::cairo_lang_macro::PostProcessContext) = #item_name;
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

fn create_attribute(input: TokenStream, attr_prefix: &str, callback_prefix: &str) -> TokenStream {
    let input: LitStr = parse_macro_input!(input as LitStr);
    let callback_link = format!(
        "{}_DESERIALIZE{}",
        callback_prefix.to_uppercase(),
        input.value().to_uppercase()
    );
    let callback_link = syn::Ident::new(&callback_link, input.span());
    let item_name = format!("{}{}", attr_prefix, input.value());
    let org_name = syn::Ident::new(&item_name, input.span());

    let expanded = quote! {
        fn #org_name() {
            // No-op function to prevent name conflicts.
        }

        #[::cairo_lang_macro::linkme::distributed_slice(::cairo_lang_macro::MACRO_DEFINITIONS_SLICE)]
        #[linkme(crate = ::cairo_lang_macro::linkme)]
        static #callback_link: ::cairo_lang_macro::ExpansionDefinition = ::cairo_lang_macro::ExpansionDefinition {
            name: #item_name,
            doc: "",
            kind: ::cairo_lang_macro::ExpansionKind::Attr,
            fun: ::cairo_lang_macro::ExpansionFunc::Attr(::cairo_lang_macro::no_op_attr),
        };
    };
    TokenStream::from(expanded)
}

const EXEC_ATTR_PREFIX: &str = "__exec_attr_";
const NO_OP_ATTR_PREFIX: &str = "__no_op_attr_";

#[proc_macro]
pub fn executable_attribute(input: TokenStream) -> TokenStream {
    create_attribute(input, EXEC_ATTR_PREFIX, "EXEC_ATTR")
}

#[proc_macro]
pub fn no_op_attribute(input: TokenStream) -> TokenStream {
    create_attribute(input, NO_OP_ATTR_PREFIX, "NO_OP_ATTR")
}
