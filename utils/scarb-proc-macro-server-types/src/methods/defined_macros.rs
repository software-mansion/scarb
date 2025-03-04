use crate::scope::Package;

use super::Method;
use serde::{Deserialize, Serialize};

/// Response structure containing a listed information
/// about the macros used by packages from the workspace.
///
/// # Invariant
/// Each [`PackageDefinedMacrosInfo`] in `macros_for_packages` should have
/// an exclusive `package` field which identifies it in the response.
/// Effectively, it simulates a HashMap which cannot be used directly
/// because of the JSON serialization.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DefinedMacrosResponse {
    /// A list of [`PackageDefinedMacrosInfo`], describing macros
    /// available for each package from the workspace.
    pub macros_for_packages: Vec<PackageDefinedMacrosInfo>,
}

/// Response structure containing lists of all defined macros available for one package.
/// Details the types of macros that can be expanded, such as attributes, inline macros, and derives.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PackageDefinedMacrosInfo {
    /// A package for which the macros are defined.
    /// It should identify [`PackageDefinedMacrosInfo`]
    /// uniquely in the [`DefinedMacrosResponse`].
    pub package: Package,
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
