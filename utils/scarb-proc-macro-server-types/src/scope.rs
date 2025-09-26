use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Representation of the Scarb package.
///
/// # Invariants
/// 1. (`name`, `discriminator`) pair must represent the package uniquely in the workspace.
/// 2. `name` and `discriminator` must refer to the same `CompilationUnitComponent` and must be identical to those from `scarb-metadata`.
///    At the moment, they are obtained using `CompilationUnitComponent::cairo_package_name`
///    and `CompilationUnitComponentId::to_discriminator`, respectively.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct CompilationUnitComponent {
    /// Name of the `CompilationUnitComponent` associated with the package.
    pub name: String,
    /// A `CompilationUnitComponent` discriminator.
    /// `None` only for corelib.
    pub discriminator: Option<String>,
}

impl CompilationUnitComponent {
    /// Builds a new [`CompilationUnitComponent`] from `name` and `discriminator`
    /// without checking for consistency between them and with the metadata.
    ///
    /// # Safety
    /// Communication between PMS and LS relies on the invariant that `name` and `discriminator`
    /// refer to the same CU component and are consistent with `scarb-metadata`.
    /// The caller must ensure correctness of the provided values.
    pub fn new(name: impl AsRef<str>, discriminator: impl AsRef<str>) -> Self {
        Self {
            name: name.as_ref().to_string(),
            discriminator: Some(discriminator.as_ref().to_string()),
        }
    }
}

/// A Scarb workspace.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct Workspace {
    /// A path of the workspace's manifest.
    pub manifest_path: PathBuf,
}

/// A description of the location in the workspace where particular macro is available.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct ProcMacroScope {
    /// A Scarb workspace in which the action occurs.
    pub workspace: Workspace,
    /// A [`CompilationUnitComponent`] in which context the action occurs.
    pub component: CompilationUnitComponent,
}
