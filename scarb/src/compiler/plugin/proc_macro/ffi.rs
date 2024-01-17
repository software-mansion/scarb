use crate::core::{Package, PackageId};
use anyhow::Result;

/// Representation of a single procedural macro.
///
/// This struct is a wrapper around a shared library containing the procedural macro implementation.
/// It is responsible for loading the shared library and providing a safe interface for code expansion.
#[derive(Debug, Clone)]
pub struct ProcMacroInstance {
    package_id: PackageId,
}

impl ProcMacroInstance {
    pub fn try_new(package: Package) -> Result<Self> {
        // Load shared library
        // TODO(maciektr): Implement
        Ok(Self {
            package_id: package.id,
        })
    }

    pub fn declared_attributes(&self) -> Vec<String> {
        vec![self.package_id.name.to_string()]
    }
}
