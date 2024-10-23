use super::Method;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DefinedMacrosResponse {
    pub attributes: Vec<String>,
    pub inline_macros: Vec<String>,
    pub derives: Vec<String>,
    pub executables: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DefinedMacrosParams {}

pub struct DefinedMacros;

impl Method for DefinedMacros {
    const METHOD: &'static str = "definedMacros";

    type Params = DefinedMacrosParams;
    type Response = DefinedMacrosResponse;
}
