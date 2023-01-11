use std::collections::HashMap;

use anyhow::Result;

use crate::core::package::{Package, PackageId};
use crate::core::registry::cache::RegistryCache;
use crate::core::registry::Registry;
use crate::core::resolver::Resolve;
use crate::core::workspace::Workspace;
use crate::internal::asyncx::AwaitSync;
use crate::resolver;

pub struct WorkspaceResolution {
    pub resolve: Resolve,
    pub packages: HashMap<PackageId, Package>,
}

/// Resolves workspace dependencies and downloads missing packages.
#[tracing::instrument(
    level = "debug",
    skip_all,
    fields(root = ws.root().display().to_string())
)]
pub fn resolve_workspace(ws: &Workspace<'_>) -> Result<WorkspaceResolution> {
    let registry = Registry::preloaded(ws.members(), ws.config());
    let mut registry_cache = RegistryCache::new(registry);

    let members_summaries = ws
        .members()
        .map(|pkg| pkg.manifest.summary.clone())
        .collect::<Vec<_>>();

    let resolve = resolver::resolve(&members_summaries, &mut registry_cache, ws.config())?;

    // Gather [`Package`] instances from this resolver result, by asking the [`RegistryCache`]
    // to download resolved packages.
    //
    // Currently, it is expected that all packages are already downloaded during resolution,
    // so the `download` calls in this method should be cheap, but this may change the future.
    let packages =
        async_collect_packages_from_resolve_graph(&resolve, &mut registry_cache).await_sync()?;

    Ok(WorkspaceResolution { resolve, packages })
}

async fn async_collect_packages_from_resolve_graph(
    resolve: &Resolve,
    registry: &mut RegistryCache<'_>,
) -> Result<HashMap<PackageId, Package>> {
    let mut packages = HashMap::with_capacity(resolve.package_ids().size_hint().0);
    // TODO(mkaput): Parallelize this loop, this is tricky because RegistryCache, Registry
    //   and Source take &mut self.
    for package_id in resolve.package_ids() {
        let package = registry.download(package_id).await?;
        packages.insert(package_id, package);
    }
    Ok(packages)
}
