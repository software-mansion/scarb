use super::ProcMacroResult;
use crate::Method;
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
    const METHOD: &'static str = "expand-attribute";

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
    const METHOD: &'static str = "expand-derive";

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
    const METHOD: &'static str = "expand-inline";

    type Params = ExpandInlineMacroParams;
    type Response = ProcMacroResult;
}
