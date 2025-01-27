use serde::{Deserialize, Serialize};

/// A description of the location in project where server action is requested.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct RequestContext {
    /// ID of the Scarb CU in which context the action occurs.
    pub compilation_unit_id: String,
    /// ID of the component belonging to the compilation unit in which the action is performed.
    pub compilation_unit_component_id: String,
}
