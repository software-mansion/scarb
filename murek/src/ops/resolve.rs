use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::core::package::{Package, PackageId};
use crate::core::registry::cache::RegistryCache;
use crate::core::registry::Registry;
use crate::core::workspace::Workspace;
use crate::resolver;

pub struct WorkspaceResolution {
    pub targets: HashMap<PackageId, HashSet<PackageId>>,
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
    let packages = {
        let package_ids = Vec::from_iter(resolve.package_ids.iter().copied());
        let packages = registry_cache.download_many(&package_ids)?;
        HashMap::from_iter(packages.into_iter().map(|pkg| (pkg.id, pkg)))
    };

    Ok(WorkspaceResolution {
        targets: resolve.targets,
        packages,
    })
}
