use anyhow::Result;
use async_trait::async_trait;
use tracing::debug;

use crate::core::registry::patch_map::PatchMap;
use crate::core::registry::Registry;
use crate::core::{ManifestDependency, Package, PackageId, Summary};

/// Intercepts [`Registry::query`] operations to follow patches set by user.
pub struct RegistryPatcher<'a> {
    registry: &'a dyn Registry,
    patch_map: &'a PatchMap,
}

impl<'a> RegistryPatcher<'a> {
    pub fn new(registry: &'a dyn Registry, patch_map: &'a PatchMap) -> Self {
        Self {
            registry,
            patch_map,
        }
    }
}

#[async_trait(?Send)]
impl<'a> Registry for RegistryPatcher<'a> {
    #[tracing::instrument(skip_all)]
    async fn query(&self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
        let patch = self.patch_map.lookup(dependency);

        if patch != dependency {
            debug!(%dependency, %patch);
        }

        self.registry.query(patch).await
    }

    async fn download(&self, package_id: PackageId) -> Result<Package> {
        self.registry.download(package_id).await
    }
}
