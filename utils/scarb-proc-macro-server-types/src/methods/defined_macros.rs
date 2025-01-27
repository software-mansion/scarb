use std::collections::HashMap;

use super::Method;
use serde::{Deserialize, Serialize};

/// Response structure containing a mapping from package names to the information about the macros they use.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DefinedMacrosResponse {
    pub workspace_macro_info: HashMap<String, CompilationUnitDefinedMacrosInfo>,
}

/// Response structure containing lists of all defined macros available
/// for particular compilation units in the workspace.
///
/// Details the types of macros that can be expanded, such as attributes, inline macros, and derives.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompilationUnitDefinedMacrosInfo {
    /// List of attribute macro names available.
    pub attributes: Vec<String>,
    /// List of inline macro names available.
    pub inline_macros: Vec<String>,
    /// List of derive macro identifiers available.
    pub derives: Vec<String>,
    /// List of executable attributes available.
    pub executables: Vec<String>,
}

/// Parameters for the request to retrieve all defined macros.
///
/// This is typically used as the initial query to understand which macros are supported.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DefinedMacrosParams {}

/// Represents a request to retrieve information on all macros defined and supported.
///
/// This request is typically sent as the initial query to understand which macros are supported.
pub struct DefinedMacros;

impl Method for DefinedMacros {
    const METHOD: &'static str = "definedMacros";

    type Params = DefinedMacrosParams;
    type Response = DefinedMacrosResponse;
}
