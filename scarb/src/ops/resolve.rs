use std::collections::HashMap;

use anyhow::Result;

use crate::compiler::{CompilationUnit, Profile};
use crate::core::package::{Package, PackageId};
use crate::core::registry::cache::RegistryCache;
use crate::core::registry::source_map::SourceMap;
use crate::core::registry::Registry;
use crate::core::resolver::Resolve;
use crate::core::workspace::Workspace;
use crate::internal::asyncx::AwaitSync;
use crate::resolver;

pub struct WorkspaceResolve {
    pub resolve: Resolve,
    pub packages: HashMap<PackageId, Package>,
}

impl WorkspaceResolve {
    pub fn package_components_of(
        &self,
        root_package: PackageId,
    ) -> impl Iterator<Item = Package> + '_ {
        assert!(self.packages.contains_key(&root_package));
        self.resolve
            .package_components_of(root_package)
            .map(|id| self.packages[&id].clone())
    }
}

/// Resolves workspace dependencies and downloads missing packages.
#[tracing::instrument(
    level = "debug",
    skip_all,
    fields(root = ws.root().to_string())
)]
pub fn resolve_workspace(ws: &Workspace<'_>) -> Result<WorkspaceResolve> {
    async {
        let source_map = SourceMap::preloaded(ws.members(), ws.config());
        let mut registry_cache = RegistryCache::new(source_map);

        let members_summaries = ws
            .members()
            .map(|pkg| pkg.manifest.summary.clone())
            .collect::<Vec<_>>();

        let resolve = resolver::resolve(&members_summaries, &mut registry_cache).await?;

        let packages = collect_packages_from_resolve_graph(&resolve, &mut registry_cache).await?;

        Ok(WorkspaceResolve { resolve, packages })
    }
    .await_sync()
}

#[tracing::instrument(skip_all, level = "debug")]
pub fn generate_compilation_units(
    resolve: &WorkspaceResolve,
    ws: &Workspace<'_>,
) -> Result<Vec<CompilationUnit>> {
    let mut units = Vec::with_capacity(ws.members().size_hint().0);
    for member in ws.members() {
        let components = resolve.package_components_of(member.id).collect::<Vec<_>>();
        for target in &member.manifest.targets {
            let unit = CompilationUnit {
                package: member.clone(),
                target: target.clone(),
                components: components.clone(),
                // TODO(mkaput): Support defining profiles in manifest.
                profile: Profile {
                    name: "release".into(),
                },
            };
            units.push(unit);
        }
    }
    Ok(units)
}

/// Gather [`Package`] instances from this resolver result, by asking the [`RegistryCache`]
/// to download resolved packages.
///
/// Currently, it is expected that all packages are already downloaded during resolution,
/// so the `download` calls in this method should be cheap, but this may change the future.
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
