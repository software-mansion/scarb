use crate::core::Package;

/// Representation of a single procedural macro.
///
/// This struct is a wrapper around a shared library containing the procedural macro implementation.
/// It is responsible for loading the shared library and providing a safe interface for code expansion.
#[derive(Debug, Clone)]
pub struct ProcMacroInstance {}

impl ProcMacroInstance {
    pub fn try_new(_package: Package) -> anyhow::Result<Self> {
        // Load shared library
        // TODO(maciektr): Implement
        Ok(Self {})
    }
}
