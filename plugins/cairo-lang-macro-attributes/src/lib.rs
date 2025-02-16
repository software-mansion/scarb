use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use scarb_stable_hash::short_hash;
use syn::spanned::Spanned;
use syn::{
    parse::Parse, parse::ParseStream, parse_macro_input, Expr, Ident, ItemFn, LitStr, Meta, Result,
    Token,
};

/// Constructs the attribute macro implementation.
///
/// This macro hides the conversion to stable ABI structs from the user.
///
/// Note, that this macro can be used multiple times, to define multiple independent attribute macros.
#[proc_macro_attribute]
pub fn attribute_macro(args: TokenStream, input: TokenStream) -> TokenStream {
    macro_helper(
        input,
        parse_macro_input!(args as AttributeArgs),
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
pub fn inline_macro(args: TokenStream, input: TokenStream) -> TokenStream {
    // Emit compilation error if `parent` argument is used.
    let attribute_args = parse_macro_input!(args as AttributeArgs);
    if let Some(path) = attribute_args.parent_module_path {
        return syn::Error::new(path.span(), "inline macro cannot use `parent` argument")
            .to_compile_error()
            .into();
    }
    // Otherwise, proceed with the macro expansion.
    macro_helper(
        input,
        Default::default(),
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
pub fn derive_macro(args: TokenStream, input: TokenStream) -> TokenStream {
    macro_helper(
        input,
        parse_macro_input!(args as AttributeArgs),
        quote!(::cairo_lang_macro::ExpansionKind::Derive),
        quote!(::cairo_lang_macro::ExpansionFunc::Other),
    )
}

fn macro_helper(
    input: TokenStream,
    args: AttributeArgs,
    kind: impl ToTokens,
    func: impl ToTokens,
) -> TokenStream {
    let item: ItemFn = parse_macro_input!(input as ItemFn);

    let original_item_name = item.sig.ident.to_string();
    let expansion_name = if let Some(path) = args.parent_module_path {
        let value = path.value();
        if !is_valid_path(&value) {
            return syn::Error::new(path.span(), "`parent` argument is not a valid path")
                .to_compile_error()
                .into();
        }
        format!("{}::{}", value, original_item_name)
    } else {
        original_item_name
    };
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

    let callback_link = Ident::new(callback_link.as_str(), item.span());

    let expanded = quote! {
        #item

        #[::cairo_lang_macro::linkme::distributed_slice(::cairo_lang_macro::MACRO_DEFINITIONS_SLICE)]
        #[linkme(crate = ::cairo_lang_macro::linkme)]
        static #callback_link: ::cairo_lang_macro::ExpansionDefinition =
            ::cairo_lang_macro::ExpansionDefinition{
                name: #expansion_name,
                doc: #doc,
                kind: #kind,
                fun: #func(#item_name),
            };
    };
    TokenStream::from(expanded)
}

#[derive(Default)]
struct AttributeArgs {
    parent_module_path: Option<LitStr>,
}

impl Parse for AttributeArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Ok(Self {
                parent_module_path: None,
            });
        }
        let parent_identifier: Ident = input.parse()?;
        if parent_identifier != "parent" {
            return Err(input.error("only `parent` argument is supported"));
        }
        let _eq_token: Token![=] = input.parse()?;
        let parent_module_path: LitStr = input.parse()?;
        Ok(Self {
            parent_module_path: Some(parent_module_path),
        })
    }
}

fn is_valid_path(path: &str) -> bool {
    let mut chars = path.chars().peekable();
    let mut last_was_colon = false;
    while let Some(c) = chars.next() {
        if c.is_alphanumeric() || c == '_' {
            last_was_colon = false;
        } else if c == ':' {
            if last_was_colon {
                // If the last character was also a colon, continue
                last_was_colon = false;
            } else {
                // If the next character is not a colon, it's an error
                if chars.peek() != Some(&':') {
                    return false;
                }
                last_was_colon = true;
            }
        } else {
            return false;
        }
    }
    // If the loop ends with a colon flag still true, it means the string ended with a single colon.
    !last_was_colon
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
    let callback_link = Ident::new(callback_link.as_str(), item.span());

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
    item.sig.ident = Ident::new(item_name.as_str(), item.sig.ident.span());
    item
}

const EXEC_ATTR_PREFIX: &str = "__exec_attr_";

#[proc_macro]
pub fn executable_attribute(input: TokenStream) -> TokenStream {
    let input: LitStr = parse_macro_input!(input as LitStr);
    let callback_link = format!("EXEC_ATTR_DESERIALIZE{}", input.value().to_uppercase());
    let callback_link = Ident::new(callback_link.as_str(), input.span());
    let item_name = format!("{EXEC_ATTR_PREFIX}{}", input.value());
    let org_name = Ident::new(item_name.as_str(), input.span());
    let expanded = quote! {
        fn #org_name() {
            // No op to ensure no function with the same name is created.
        }

        #[::cairo_lang_macro::linkme::distributed_slice(::cairo_lang_macro::MACRO_DEFINITIONS_SLICE)]
        #[linkme(crate = ::cairo_lang_macro::linkme)]
        static #callback_link: ::cairo_lang_macro::ExpansionDefinition =
            ::cairo_lang_macro::ExpansionDefinition{
                name: #item_name,
                doc: "",
                kind: ::cairo_lang_macro::ExpansionKind::Attr,
                fun: ::cairo_lang_macro::ExpansionFunc::Attr(::cairo_lang_macro::no_op_attr),
            };
    };
    TokenStream::from(expanded)
}
