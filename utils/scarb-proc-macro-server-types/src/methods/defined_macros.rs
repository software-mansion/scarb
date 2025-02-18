use std::collections::HashMap;

use super::Method;
use serde::{Deserialize, Serialize};

// NOTE: The keys of this mapping are seralized PackageIds.
// We also rely on the implicit contract that CompilationUnitComponentId.repr == PackageId.to_serialized_string().
//
/// Response structure containing a mapping from package IDs
/// to the information about the macros they use.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DefinedMacrosResponse {
    /// A mapping of the form: `package (as a serialized `PackageId`) -> macros info`.
    /// Contains seralized IDs of all packages from the workspace,
    /// mapped to the [`PackageDefinedMacrosInfo`], describing macros available for them.
    pub macros_by_package_id: HashMap<String, PackageDefinedMacrosInfo>,
}

/// Response structure containing lists of all defined macros available for one package.
/// Details the types of macros that can be expanded, such as attributes, inline macros, and derives.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PackageDefinedMacrosInfo {
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
