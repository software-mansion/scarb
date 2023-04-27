use std::collections::HashMap;

use anyhow::Result;
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use futures::TryFutureExt;
use itertools::Itertools;

use crate::compiler::{CompilationUnit, CompilationUnitComponent};
use crate::core::package::{Package, PackageId};
use crate::core::registry::cache::RegistryCache;
use crate::core::registry::source_map::SourceMap;
use crate::core::registry::Registry;
use crate::core::resolver::Resolve;
use crate::core::workspace::Workspace;
use crate::core::Target;
use crate::resolver;

pub struct WorkspaceResolve {
    pub resolve: Resolve,
    pub packages: HashMap<PackageId, Package>,
}

impl WorkspaceResolve {
    /// Collect all [`Package`]s needed to compile a root package.
    ///
    /// Returns a collection of all [`Package`]s of packages needed to provide as _crate roots_
    /// to the Cairo compiler, or to load as _cairo plugins_, in order to build a particular
    /// package (named _root package_).
    ///
    /// # Safety
    /// * Asserts that `root_package` is a node in this graph.
    pub fn solution_of(&self, root_package: PackageId) -> impl Iterator<Item = Package> + '_ {
        assert!(self.packages.contains_key(&root_package));
        self.resolve
            .solution_of(root_package)
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
    ws.config().tokio_handle().block_on(
        async {
            let source_map = SourceMap::preloaded(ws.members(), ws.config());
            let mut registry_cache = RegistryCache::new(source_map);

            let members_summaries = ws
                .members()
                .map(|pkg| pkg.manifest.summary.clone())
                .collect::<Vec<_>>();

            let resolve = resolver::resolve(&members_summaries, &mut registry_cache).await?;

            let packages =
                collect_packages_from_resolve_graph(&resolve, &mut registry_cache).await?;

            Ok(WorkspaceResolve { resolve, packages })
        }
        .into_future(),
    )
}

#[tracing::instrument(skip_all, level = "debug")]
pub fn generate_compilation_units(
    resolve: &WorkspaceResolve,
    ws: &Workspace<'_>,
) -> Result<Vec<CompilationUnit>> {
    let mut units = Vec::with_capacity(ws.members().size_hint().0);
    for member in ws.members() {
        let mut packages = resolve
            .solution_of(member.id)
            .filter(|pkg| {
                let is_self_or_lib = member.id == pkg.id || pkg.is_lib();
                // Print a warning if this dependency is not a library.
                if !is_self_or_lib {
                    ws.config().ui().warn(format!(
                        "{} ignoring invalid dependency `{}` which is missing a lib target",
                        member.id, pkg.id.name
                    ));
                }
                is_self_or_lib
            })
            .collect::<Vec<_>>();

        // Ensure the member is first element, and it is followed by `core`, to ensure the order
        // invariant of the `CompilationUnit::components` field holds.
        packages.sort_by_key(|package| {
            if package.id == member.id {
                0
            } else if package.id.is_core() {
                1
            } else {
                2
            }
        });

        for member_target in &member.manifest.targets {
            let cfg_set = build_cfg_set(member_target);

            let components = packages
                .iter()
                .cloned()
                .map(|package| {
                    // If this is this compilation's unit main package, then use the target we are
                    // building. Otherwise, assume library target for all dependency packages,
                    // because that's what it is for.
                    let target = if package.id == member.id {
                        member_target
                    } else {
                        // We can safely unwrap here, because compilation unit generator ensures
                        // that all dependencies have library target.
                        package.fetch_target(Target::LIB).unwrap()
                    };
                    let target = target.clone();

                    CompilationUnitComponent { package, target }
                })
                .collect();

            let unit = CompilationUnit {
                main_package_id: member.id,
                components,
                profile: ws.current_profile()?,
                compiler_config: member.manifest.compiler_config.clone(),
                cfg_set,
            };
            units.push(unit);
        }
    }

    assert!(
        units.iter().map(CompilationUnit::id).all_unique(),
        "All generated compilation units must have unique IDs."
    );

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
    // TODO(#6): Parallelize this loop.
    for package_id in resolve.package_ids() {
        let package = registry.download(package_id).await?;
        packages.insert(package_id, package);
    }
    Ok(packages)
}

/// Build a set of `cfg` items to enable while building the compilation unit.
fn build_cfg_set(target: &Target) -> CfgSet {
    CfgSet::from_iter([Cfg::kv("target", target.kind.clone())])
}
