use std::collections::HashMap;

use anyhow::{bail, Result};
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use futures::TryFutureExt;
use itertools::Itertools;

use crate::compiler::{CompilationUnit, CompilationUnitCairoPlugin, CompilationUnitComponent};
use crate::core::package::{Package, PackageClass, PackageId};
use crate::core::registry::cache::RegistryCache;
use crate::core::registry::patch_map::PatchMap;
use crate::core::registry::patcher::RegistryPatcher;
use crate::core::registry::source_map::SourceMap;
use crate::core::registry::Registry;
use crate::core::resolver::Resolve;
use crate::core::workspace::Workspace;
use crate::core::{DependencyVersionReq, ManifestDependency, PackageName, SourceId, Target};
use crate::internal::to_version::ToVersion;
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
            let mut patch_map = PatchMap::new();

            let cairo_version = crate::version::get().cairo.version.parse().unwrap();
            let version_req = DependencyVersionReq::exact(&cairo_version);
            patch_map.insert(
                SourceId::default().canonical_url.clone(),
                [
                    ManifestDependency {
                        name: PackageName::CORE,
                        version_req: version_req.clone(),
                        source_id: SourceId::for_std(),
                    },
                    ManifestDependency {
                        name: PackageName::STARKNET,
                        version_req: version_req.clone(),
                        source_id: SourceId::for_std(),
                    },
                ],
            );

            let source_map = SourceMap::preloaded(ws.members(), ws.config());
            let cached = RegistryCache::new(&source_map);
            let patched = RegistryPatcher::new(&cached, &patch_map);

            let members_summaries = ws
                .members()
                .map(|pkg| pkg.manifest.summary.clone())
                .collect::<Vec<_>>();

            let resolve = resolver::resolve(&members_summaries, &patched).await?;

            let packages = collect_packages_from_resolve_graph(&resolve, &patched).await?;

            Ok(WorkspaceResolve { resolve, packages })
        }
        .into_future(),
    )
}

/// Gather [`Package`] instances from this resolver result, by asking the [`RegistryCache`]
/// to download resolved packages.
///
/// Currently, it is expected that all packages are already downloaded during resolution,
/// so the `download` calls in this method should be cheap, but this may change the future.
#[tracing::instrument(level = "trace", skip_all)]
async fn collect_packages_from_resolve_graph(
    resolve: &Resolve,
    registry: &dyn Registry,
) -> Result<HashMap<PackageId, Package>> {
    let mut packages = HashMap::with_capacity(resolve.package_ids().size_hint().0);
    // TODO(#6): Parallelize this loop.
    for package_id in resolve.package_ids() {
        let package = registry.download(package_id).await?;
        packages.insert(package_id, package);
    }
    Ok(packages)
}

#[tracing::instrument(skip_all, level = "debug")]
pub fn generate_compilation_units(
    resolve: &WorkspaceResolve,
    ws: &Workspace<'_>,
) -> Result<Vec<CompilationUnit>> {
    let mut units = Vec::with_capacity(ws.members().size_hint().0);
    for member in ws.members() {
        units.extend(if member.is_cairo_plugin() {
            generate_cairo_plugin_compilation_units()?
        } else {
            generate_cairo_compilation_units(&member, resolve, ws)?
        });
    }

    assert!(
        units.iter().map(CompilationUnit::id).all_unique(),
        "All generated compilation units must have unique IDs."
    );

    Ok(units)
}

fn generate_cairo_compilation_units(
    member: &Package,
    resolve: &WorkspaceResolve,
    ws: &Workspace<'_>,
) -> Result<Vec<CompilationUnit>> {
    let mut classes = resolve.solution_of(member.id).into_group_map_by(|pkg| {
        if pkg.id == member.id {
            // Always classify the member as a library (even if it's [PackageClass::Other]),
            // so that it will end up being a component.
            assert!(!member.is_cairo_plugin());
            PackageClass::Library
        } else {
            pkg.classify()
        }
    });

    let mut packages = classes.remove(&PackageClass::Library).unwrap_or_default();
    let cairo_plugins = classes
        .remove(&PackageClass::CairoPlugin)
        .unwrap_or_default();
    let other = classes.remove(&PackageClass::Other).unwrap_or_default();

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

    assert!(!packages.is_empty());
    assert_eq!(packages[0].id, member.id);

    check_cairo_version_compatibility(&packages, ws)?;

    // Print warnings for dependencies that are not usable.
    for pkg in other {
        ws.config().ui().warn(format!(
            "{} ignoring invalid dependency `{}` which is missing a lib or cairo-plugin target",
            member.id, pkg.id.name
        ));
    }

    let cairo_plugins = cairo_plugins
        .into_iter()
        .map(|package| CompilationUnitCairoPlugin { package })
        .collect::<Vec<_>>();

    let profile = ws.current_profile()?;

    Ok(member
        .manifest
        .targets
        .iter()
        .map(|member_target| {
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

            CompilationUnit {
                main_package_id: member.id,
                components,
                cairo_plugins: cairo_plugins.clone(),
                profile: profile.clone(),
                compiler_config: member.manifest.compiler_config.clone(),
                cfg_set,
            }
        })
        .collect())
}

/// Build a set of `cfg` items to enable while building the compilation unit.
fn build_cfg_set(target: &Target) -> CfgSet {
    CfgSet::from_iter([Cfg::kv("target", target.kind.clone())])
}

fn check_cairo_version_compatibility(packages: &[Package], ws: &Workspace<'_>) -> Result<()> {
    let current_version = crate::version::get().cairo.version.to_version().unwrap();
    let matching_version = packages.iter().all(|pkg| {
        match &pkg.manifest.metadata.cairo_version {
            Some(package_version) if !package_version.matches(&current_version) => {
                ws.config().ui().error(format!(
                    "Package {}. Required Cairo version isn't compatible with current version. Should be: {} is: {}",
                    pkg.id.name, package_version, current_version
                ));
                false
            }
            _ => true
        }
    });
    if !matching_version {
        bail!("For each package, the required Cairo version must match the current Cairo version.");
    }
    Ok(())
}

fn generate_cairo_plugin_compilation_units() -> Result<Vec<CompilationUnit>> {
    bail!("compiling Cairo plugin packages is not possible yet")
}
