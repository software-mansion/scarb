use std::collections::HashMap;

use super::Method;
use serde::{Deserialize, Serialize};

/// Response structure containing a mapping from package IDs
/// to the information about the macros they use.
///
/// # Invariant
/// Correct usage of this struct during proc macro server <-> LS communication
/// relies on the implicit contract that keys of `macros_by_package_id` are of form
/// `PackageId.to_serialized_string()` which is always equal to
/// `scarb_metadata::CompilationUnitComponentId.repr`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DefinedMacrosResponse {
    /// A mapping of the form: `package (as a serialized `PackageId`) -> macros info`.
    /// Contains serialized IDs of all packages from the workspace,
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
