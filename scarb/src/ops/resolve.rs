use std::collections::HashMap;

use anyhow::Result;

use crate::core::manifest::ManifestMetadata;
use crate::core::package::{Package, PackageId};
use crate::core::registry::cache::RegistryCache;
use crate::core::registry::source_map::SourceMap;
use crate::core::registry::Registry;
use crate::core::resolver::Resolve;
use crate::core::source::SourceId;
use crate::core::workspace::Workspace;
use crate::core::{Manifest, Summary};
use crate::internal::asyncx::AwaitSync;
use crate::resolver;
use clap::crate_version;

pub struct WorkspaceResolve {
    pub resolve: Resolve,
    pub packages: HashMap<PackageId, Package>,
}

/// Resolves workspace dependencies and downloads missing packages.
#[tracing::instrument(
    level = "debug",
    skip_all,
    fields(root = ws.root().display().to_string())
)]
pub fn resolve_workspace(ws: &Workspace<'_>) -> Result<WorkspaceResolve> {
    async {
        let packages = ws.members();
        let corelib_package: Package = corelib_package(ws).unwrap();

        let source_map = SourceMap::preloaded(packages.chain([corelib_package]), ws.config());
        let mut registry_cache = RegistryCache::new(source_map);

        let members_summaries = ws
            .members()
            .map(|pkg| pkg.manifest.summary.clone())
            .collect::<Vec<_>>();

        let resolve = resolver::resolve(&members_summaries, &mut registry_cache).await?;

        // Gather [`Package`] instances from this resolver result, by asking the [`RegistryCache`]
        // to download resolved packages.
        //
        // Currently, it is expected that all packages are already downloaded during resolution,
        // so the `download` calls in this method should be cheap, but this may change the future.
        let packages = collect_packages_from_resolve_graph(&resolve, &mut registry_cache).await?;

        Ok(WorkspaceResolve { resolve, packages })
    }
    .await_sync()
}

fn corelib_package(ws: &Workspace<'_>) -> Result<Package> {
    let version = crate_version!();
    let source_id = SourceId::for_corelib(version).unwrap();
    let corelib_path = ws.config().dirs.registry_src_dir.join("corelib");
    let package_id = PackageId::new("corelib", version, source_id.clone()).unwrap();
    let manifest = Manifest {
        summary: Summary::new(package_id, Vec::new()),
        metadata: ManifestMetadata {
            authors: None,
            urls: None,
            custom_metadata: None,
            description: None,
            documentation: None,
            homepage: None,
            keywords: None,
            license: None,
            license_file: None,
            readme: None,
            repository: None,
        },
    };

    Ok(Package::new(
        package_id,
        corelib_path.join("cairo_project.toml"),
        Box::new(manifest),
    ))
}

#[tracing::instrument(level = "trace", skip_all)]
async fn collect_packages_from_resolve_graph(
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
