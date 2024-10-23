use super::Method;
use super::ProcMacroResult;
use cairo_lang_macro::TokenStream;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct ExpandAttributeParams {
    pub attr: String,
    pub args: TokenStream,
    pub item: TokenStream,
}

pub struct ExpandAttribute;

impl Method for ExpandAttribute {
    const METHOD: &'static str = "expandAttribute";

    type Params = ExpandAttributeParams;
    type Response = ProcMacroResult;
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct ExpandDeriveParams {
    pub derives: Vec<String>,
    pub item: TokenStream,
}

pub struct ExpandDerive;

impl Method for ExpandDerive {
    const METHOD: &'static str = "expandDerive";

    type Params = ExpandDeriveParams;
    type Response = ProcMacroResult;
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct ExpandInlineMacroParams {
    pub name: String,
    pub item: TokenStream,
}

pub struct ExpandInline;

impl Method for ExpandInline {
    const METHOD: &'static str = "expandInline";

    type Params = ExpandInlineMacroParams;
    type Response = ProcMacroResult;
}
