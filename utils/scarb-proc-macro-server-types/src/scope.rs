use serde::{Deserialize, Serialize};

/// A description of the location in the workspace where particular macro is available.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct ProcMacroScope {
    /// Serialized `CompilationUnitComponentId` of the compilation unit's main component
    /// in which context the action occurs.
    pub compilation_unit_main_component_id: String,
}
