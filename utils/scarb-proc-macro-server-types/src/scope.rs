use serde::{Deserialize, Serialize};

/// A description of the location in the workspace where particular macro is available.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct ProcMacroScope {
    /// Serialized `PackageId` of the package in which context the action occurs.
    pub package_id: String,
}
