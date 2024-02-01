use std::collections::HashMap;

use anyhow::{bail, Result};
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use futures::TryFutureExt;
use indoc::formatdoc;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::compiler::{CompilationUnit, CompilationUnitCairoPlugin, CompilationUnitComponent};
use crate::core::lockfile::Lockfile;
use crate::core::package::{Package, PackageClass, PackageId};
use crate::core::registry::cache::RegistryCache;
use crate::core::registry::patch_map::PatchMap;
use crate::core::registry::patcher::RegistryPatcher;
use crate::core::registry::source_map::SourceMap;
use crate::core::registry::Registry;
use crate::core::resolver::Resolve;
use crate::core::workspace::Workspace;
use crate::core::{
    DepKind, DependencyVersionReq, ManifestDependency, PackageName, SourceId, Target, TargetKind,
    TestTargetProps, TestTargetType,
};
use crate::internal::to_version::ToVersion;
use crate::ops::lockfile::{read_lockfile, write_lockfile};
use crate::{resolver, DEFAULT_SOURCE_PATH};

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
    pub fn solution_of(&self, root_package: PackageId, target_kind: &TargetKind) -> Vec<Package> {
        assert!(self.packages.contains_key(&root_package));
        self.resolve
            .solution_of(root_package, target_kind)
            .iter()
            .map(|id| self.packages[id].clone())
            .collect_vec()
    }
}

#[derive(Debug, Default)]
pub struct ResolveOpts {
    /// Do not use lockfile when resolving.
    pub update: bool,
}

pub fn resolve_workspace(ws: &Workspace<'_>) -> Result<WorkspaceResolve> {
    let opts: ResolveOpts = Default::default();
    resolve_workspace_with_opts(ws, &opts)
}

/// Resolves workspace dependencies and downloads missing packages.
#[tracing::instrument(level = "debug", skip_all, fields(root = ws.root().to_string()))]
pub fn resolve_workspace_with_opts(
    ws: &Workspace<'_>,
    opts: &ResolveOpts,
) -> Result<WorkspaceResolve> {
    ws.config().tokio_handle().block_on(
        async {
            let mut patch_map = PatchMap::new();

            let cairo_version = crate::version::get().cairo.version.parse().unwrap();
            let version_req = DependencyVersionReq::exact(&cairo_version);
            patch_map.insert(
                SourceId::default().canonical_url.clone(),
                [
                    ManifestDependency::builder()
                        .name(PackageName::CORE)
                        .source_id(SourceId::for_std())
                        .version_req(version_req.clone())
                        .build(),
                    ManifestDependency::builder()
                        .name(PackageName::STARKNET)
                        .version_req(version_req.clone())
                        .source_id(SourceId::for_std())
                        .build(),
                    ManifestDependency::builder()
                        .kind(DepKind::Target(TargetKind::TEST))
                        .name(PackageName::TEST_PLUGIN)
                        .version_req(version_req.clone())
                        .source_id(SourceId::for_std())
                        .build(),
                ],
            );
            if let Some(custom_source_patches) = ws.config().custom_source_patches() {
                patch_map.insert(
                    SourceId::default().canonical_url.clone(),
                    custom_source_patches.clone(),
                );
            }

            let source_map = SourceMap::preloaded(ws.members(), ws.config());
            let cached = RegistryCache::new(&source_map);
            let patched = RegistryPatcher::new(&cached, &patch_map);

            let members_summaries = ws
                .members()
                .map(|pkg| pkg.manifest.summary.clone())
                .collect::<Vec<_>>();

            let lockfile: Lockfile = if opts.update {
                Lockfile::new([])
            } else {
                read_lockfile(ws)?
            };

            let resolve =
                resolver::resolve(&members_summaries, &patched, lockfile, ws.config().ui()).await?;

            write_lockfile(Lockfile::from_resolve(&resolve), ws)?;

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
            generate_cairo_plugin_compilation_units(&member, ws)?
        } else {
            generate_cairo_compilation_units(&member, resolve, ws)?
        });
    }

    let cairo_plugins = units
        .iter()
        .flat_map(|unit| unit.cairo_plugins.clone())
        .filter(|plugin| !plugin.builtin)
        .map(|plugin| plugin.package.clone())
        .unique_by(|plugin| plugin.id)
        .collect_vec();

    for plugin in cairo_plugins {
        units.extend(generate_cairo_plugin_compilation_units(&plugin, ws)?);
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
    let profile = ws.current_profile()?;
    let mut solution = PackageSolutionCollector::new(member, resolve, ws);
    member
        .manifest
        .targets
        .iter()
        .sorted_by_key(|target| target.kind.clone())
        .map(|member_target| {
            solution.collect(&member_target.kind)?;
            let packages = solution.packages.as_ref().unwrap();
            let cairo_plugins = solution.cairo_plugins.as_ref().unwrap();

            let cfg_set = build_cfg_set(member_target);

            let props: TestTargetProps = member_target.props()?;
            let is_integration_test = props.test_type == TestTargetType::Integration;
            let test_package_id = member.id.for_test_target(member_target.name.clone());

            let mut components: Vec<CompilationUnitComponent> = packages
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
                        package.fetch_target(&TargetKind::LIB).unwrap()
                    };
                    let target = target.clone();

                    // For integration tests target, rewrite package with prefixed name.
                    // This allows integration test code to reference main package as dependency.
                    let package = if package.id == member.id && is_integration_test {
                        Package::new(
                            test_package_id,
                            package.manifest_path().to_path_buf(),
                            package.manifest.clone(),
                        )
                    } else {
                        package
                    };

                    let cfg_set = {
                        if package.id == member.id {
                            None
                        } else {
                            let component_cfg_set = cfg_set
                                .iter()
                                .filter(|cfg| **cfg != Cfg::name("test"))
                                .cloned()
                                .collect();

                            if component_cfg_set != cfg_set {
                                Some(component_cfg_set)
                            } else {
                                None
                            }
                        }
                    };

                    CompilationUnitComponent {
                        package,
                        target,
                        cfg_set,
                    }
                })
                .collect();

            // Apply overrides for integration test.
            let main_package_id = if is_integration_test {
                // Try pulling from targets.
                let target = member
                    .fetch_target(&TargetKind::LIB)
                    .cloned()
                    .unwrap_or_else(|_| {
                        // If not defined, create a dummy `lib` target.
                        Target::without_params(
                            TargetKind::LIB,
                            member.id.name.clone(),
                            member.root().join(DEFAULT_SOURCE_PATH.as_path()),
                        )
                    });

                // Add `lib` target for tested package, to be available as dependency.
                components.push(CompilationUnitComponent {
                    package: member.clone(),
                    cfg_set: None,
                    target,
                });

                // Set test package as main package for this compilation unit.
                test_package_id
            } else {
                member.id
            };

            Ok(CompilationUnit {
                main_package_id,
                components,
                cairo_plugins: cairo_plugins.clone(),
                profile: profile.clone(),
                compiler_config: member.manifest.compiler_config.clone(),
                cfg_set,
            })
        })
        .collect::<Result<Vec<CompilationUnit>>>()
}

/// Properties that can be defined on Cairo plugin target.
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
struct CairoPluginProps {
    /// Mark this macro plugin as builtin.
    /// Builtin plugins are assumed to be available in `CairoPluginRepository` for the whole Scarb execution.
    pub builtin: bool,
}

pub struct PackageSolutionCollector<'a> {
    member: &'a Package,
    resolve: &'a WorkspaceResolve,
    ws: &'a Workspace<'a>,
    packages: Option<Vec<Package>>,
    cairo_plugins: Option<Vec<CompilationUnitCairoPlugin>>,
    target_kind: Option<TargetKind>,
}

impl<'a> PackageSolutionCollector<'a> {
    pub fn new(member: &'a Package, resolve: &'a WorkspaceResolve, ws: &'a Workspace<'a>) -> Self {
        Self {
            member,
            resolve,
            ws,
            packages: None,
            cairo_plugins: None,
            target_kind: None,
        }
    }

    pub fn collect(&mut self, target_kind: &TargetKind) -> Result<()> {
        // Do not traverse graph for each target of the same kind.
        if !self
            .target_kind
            .as_ref()
            .map(|tk| tk == target_kind)
            .unwrap_or(false)
        {
            let (p, c) = self.pull_from_graph(target_kind)?;
            self.packages = Some(p.clone());
            self.cairo_plugins = Some(c.clone());
            self.target_kind = Some(target_kind.clone());
        }
        Ok(())
    }

    fn pull_from_graph(
        &self,
        target_kind: &TargetKind,
    ) -> Result<(Vec<Package>, Vec<CompilationUnitCairoPlugin>)> {
        let mut classes = self
            .resolve
            .solution_of(self.member.id, target_kind)
            .into_iter()
            .into_group_map_by(|pkg| {
                if pkg.id == self.member.id {
                    // Always classify the member as a library (even if it's [PackageClass::Other]),
                    // so that it will end up being a component.
                    assert!(!self.member.is_cairo_plugin());
                    PackageClass::Library
                } else {
                    pkg.classify()
                }
            });

        let mut packages = classes.remove(&PackageClass::Library).unwrap_or_default();
        let cairo_plugins = classes
            .remove(&PackageClass::CairoPlugin)
            .unwrap_or_default();

        // Ensure the member is first element, and it is followed by `core`, to ensure the order
        // invariant of the `CompilationUnit::components` field holds.
        packages.sort_by_key(|package| {
            if package.id == self.member.id {
                0
            } else if package.id.is_core() {
                1
            } else {
                2
            }
        });

        assert!(!packages.is_empty());
        assert_eq!(packages[0].id, self.member.id);

        check_cairo_version_compatibility(&packages, self.ws)?;

        let cairo_plugins = cairo_plugins
            .into_iter()
            .map(|package| {
                // We can safely unwrap as all packages with `PackageClass::CairoPlugin` must define plugin target.
                let target = package.target(&TargetKind::CAIRO_PLUGIN).unwrap();
                let props: CairoPluginProps = target.props()?;
                Ok(CompilationUnitCairoPlugin::builder()
                    .package(package)
                    .builtin(props.builtin)
                    .build())
            })
            .collect::<Result<Vec<_>>>()?;

        Ok((packages, cairo_plugins))
    }
}

/// Build a set of `cfg` items to enable while building the compilation unit.
fn build_cfg_set(target: &Target) -> CfgSet {
    let mut cfg = CfgSet::from_iter([Cfg::kv("target", target.kind.clone())]);
    if target.is_test() {
        cfg.insert(Cfg::name("test"));
    }
    cfg
}

fn check_cairo_version_compatibility(packages: &[Package], ws: &Workspace<'_>) -> Result<()> {
    let current_version = crate::version::get().cairo.version.to_version().unwrap();
    let matching_version = packages
        .iter()
        .all(|pkg| match &pkg.manifest.metadata.cairo_version {
            Some(package_version) if !package_version.matches(&current_version) => {
                ws.config().ui().error(formatdoc!(
                    r"
                    the required Cairo version of package {} is not compatible with current version
                    Cairo version required: {}
                    Cairo version of Scarb: {}
                    ",
                    pkg.id.name,
                    package_version,
                    current_version
                ));
                false
            }
            _ => true,
        });
    if !matching_version {
        bail!("the required Cairo version of each package must match the current Cairo version");
    }
    Ok(())
}

fn generate_cairo_plugin_compilation_units(
    member: &Package,
    ws: &Workspace<'_>,
) -> Result<Vec<CompilationUnit>> {
    Ok(vec![CompilationUnit {
        main_package_id: member.id,
        components: vec![CompilationUnitComponent {
            package: member.clone(),
            cfg_set: None,
            target: member
                .fetch_target(&TargetKind::CAIRO_PLUGIN)
                .cloned()
                // Safe to unwrap, as member.is_cairo_plugin() has been ensured before.
                .expect("main component of procedural macro must define `cairo-plugin` target"),
        }],
        cairo_plugins: Vec::new(),
        profile: ws.current_profile()?,
        compiler_config: member.manifest.compiler_config.clone(),
        cfg_set: Default::default(),
    }])
}
