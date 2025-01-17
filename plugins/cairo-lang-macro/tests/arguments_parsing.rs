use cairo_lang_macro::{attribute_macro, ProcMacroResult, TokenStream, MACRO_DEFINITIONS_SLICE};
use cairo_lang_macro_attributes::derive_macro;

#[attribute_macro]
fn t1(_a: TokenStream, _b: TokenStream) -> ProcMacroResult {
    ProcMacroResult::new(TokenStream::empty())
}

#[attribute_macro(parent = "parent_1::module")]
fn t2(_a: TokenStream, _b: TokenStream) -> ProcMacroResult {
    ProcMacroResult::new(TokenStream::empty())
}

#[attribute_macro(parent = "::parent")]
fn t3(_a: TokenStream, _b: TokenStream) -> ProcMacroResult {
    ProcMacroResult::new(TokenStream::empty())
}

#[derive_macro(parent = "parent")]
fn t4(_a: TokenStream) -> ProcMacroResult {
    ProcMacroResult::new(TokenStream::empty())
}

#[test]
fn happy_path() {
    let list: Vec<String> = MACRO_DEFINITIONS_SLICE
        .iter()
        .map(|m| m.name.to_string())
        .collect();
    assert_eq!(
        list,
        vec!["t1", "parent_1::module::t2", "::parent::t3", "parent::t4"]
    );
}

#[test]
fn test_parsing_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/args/args_*.rs");
}
