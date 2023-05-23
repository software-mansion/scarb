use proc_macro::TokenStream;
use std::path::Path;

use camino::Utf8PathBuf;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemFn};

#[proc_macro_attribute]
pub fn test_for_each_example(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item: ItemFn = parse_macro_input!(input as ItemFn);

    let examples_dir_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples");

    let legal_attr_names = [
        parse_quote!(test_for_each_example),
        parse_quote!(test_for_each_example::test_for_each_example),
    ];
    item.attrs
        .retain(|attr| !legal_attr_names.contains(attr.path()));

    let test_name = item.sig.ident.clone();

    let mut rendered_test_cases = vec![];

    for example in examples_dir_path.read_dir().unwrap() {
        let example = example.unwrap();
        if example.file_type().unwrap().is_dir() {
            let example_path = Utf8PathBuf::from_path_buf(example.path()).unwrap();

            let example_name = Ident::new(example_path.file_name().unwrap(), Span::call_site());

            let example_path = example_path.as_str();

            let test = quote! {
                #[::core::prelude::v1::test]
                fn #example_name() {
                    let example_path = ::std::path::Path::new(#example_path);
                    super::#test_name(example_path);
                }
            };

            rendered_test_cases.push(test);
        }
    }

    let expanded = quote! {
        #[allow(unused_attributes)]
        #item

        #[cfg(test)]
        mod #test_name {
            #[allow(unused_imports)]
            use super::*;

            #(#rendered_test_cases)*
        }
    };

    TokenStream::from(expanded)
}
