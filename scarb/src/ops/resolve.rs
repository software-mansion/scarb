use crate::compiler::plugin::proc_macro::{ProcMacroInstance, ProcMacroPathsProvider};
use crate::compiler::plugin::{CairoPluginProps, fetch_cairo_plugin};
use crate::compiler::{
    CairoCompilationUnit, CompilationUnit, CompilationUnitAttributes, CompilationUnitCairoPlugin,
    CompilationUnitComponent, CompilationUnitComponentId, CompilationUnitDependency,
    ComponentTarget, ProcMacroCompilationUnit, Profile,
};
use crate::core::lockfile::Lockfile;
use crate::core::package::{Package, PackageClass, PackageId};
use crate::core::registry::Registry;
use crate::core::registry::cache::RegistryCache;
use crate::core::registry::patch_map::PatchMap;
use crate::core::registry::patcher::RegistryPatcher;
use crate::core::registry::source_map::SourceMap;
use crate::core::resolver::Resolve;
use crate::core::workspace::Workspace;
use crate::core::{
    DepKind, DependencyVersionReq, FeatureName, FeaturesDefinition, ManifestCompilerConfig,
    ManifestDependency, PackageName, SourceId, Target, TargetKind, TestTargetProps, TestTargetType,
};
use crate::internal::to_version::ToVersion;
use crate::ops::lockfile::{read_lockfile, write_lockfile};
use crate::ops::{FeaturesOpts, FeaturesSelector};
use crate::{DEFAULT_SOURCE_PATH, resolver};
use anyhow::{Result, bail};
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use futures::{FutureExt, StreamExt, TryFutureExt, stream};
use indoc::formatdoc;
use itertools::Itertools;
use std::collections::{HashMap, HashSet, VecDeque};
use std::iter::zip;
use std::sync::Arc;

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

    pub fn features_unification(
        &self,
        root_package: PackageId,
        selected_features: &HashSet<FeatureName>,
        target_kind: &TargetKind,
    ) -> Result<HashMap<PackageId, HashSet<FeatureName>>> {
        assert!(self.packages.contains_key(&root_package));
        let solution = self.resolve.solution_of(root_package, target_kind);
        let mut features: HashMap<PackageId, HashSet<FeatureName>> = HashMap::default();
        features.insert(root_package, selected_features.clone());

        // Collect features enabled in manifest dependencies.
        for package_id in solution {
            let is_unit_root = root_package == package_id;
            for dep_id in self.resolve.package_dependencies_for_target_kind(
                package_id,
                target_kind,
                is_unit_root,
            ) {
                let package = self.packages.get(&package_id).unwrap();
                let dep = self.packages.get(&dep_id).unwrap();
                let summary = package.manifest.summary.clone();
                let target_kind_dependency = summary.full_dependencies().find(|md| {
                    md.name == dep_id.name && md.kind == DepKind::Target(target_kind.clone())
                });
                let normal_dependency = summary
                    .full_dependencies()
                    .find(|md| md.name == dep_id.name && md.kind == DepKind::Normal);
                for manifest_dependency in [target_kind_dependency, normal_dependency]
                    .into_iter()
                    .flatten()
                {
                    let selected_features = dep.manifest.features.select(
                        &manifest_dependency.features,
                        manifest_dependency.default_features,
                    );
                    let features = features.entry(dep_id).or_default();
                    let selected_features: HashSet<FeatureName> = selected_features.try_into()?;
                    features.extend(selected_features);
                }
            }
        }

        // Resolve features that are dependencies of selected features.
        let mut queue = VecDeque::from_iter(features.iter().flat_map(|(package_id, features)| {
            features
                .iter()
                .map(|feature| (*package_id, feature.clone()))
        }));

        while let Some((package_id, feature)) = queue.pop_front() {
            if let Some(dependants) = self
                .packages
                .get(&package_id)
                .and_then(|p| p.manifest.features.get(&feature))
            {
                for dependant in dependants.iter() {
                    let dependant_package_id = if let Some(package_name) =
                        dependant.package.as_ref()
                    {
                        let deps = self.resolve.package_dependencies_for_target_kind(
                            package_id,
                            target_kind,
                            root_package == package_id,
                        );
                        let Some(pid) = deps.iter().find(|p| p.name == *package_name) else {
                            bail!(
                                "feature `{feature}` of package `{}` depends on feature `{}` from package `{package_name}`, which is not a dependency of `{}`",
                                &package_id.name,
                                &dependant.feature,
                                &package_id.name
                            );
                        };
                        *pid
                    } else {
                        package_id
                    };
                    if !features
                        .get(&dependant_package_id)
                        .unwrap_or(&HashSet::new())
                        .contains(&dependant.feature)
                    {
                        let selected_features = features.entry(dependant_package_id).or_default();
                        selected_features.insert(dependant.feature.clone());
                        queue.push_back((dependant_package_id, dependant.feature.clone()));
                    }
                }
            }
        }

        Ok(features)
    }

    pub fn package_dependencies(
        &self,
        package_id: PackageId,
        target_kind: &TargetKind,
        main_package_id: PackageId,
    ) -> Result<Vec<Package>> {
        let is_root = main_package_id == package_id;
        assert!(self.packages.contains_key(&package_id));
        let dependencies = self
            .resolve
            .package_dependencies_for_target_kind(package_id, target_kind, is_root)
            .iter()
            .map(|id| self.packages[id].clone())
            .collect_vec();

        let re_exported = dependencies
            .iter()
            .flat_map(|dependency| {
                let package_dependencies = self
                    .resolve
                    .package_dependencies_for_target_kind(dependency.id, target_kind, is_root);
                dependency
                    .manifest
                    .summary
                    .re_export_cairo_plugins
                    .iter()
                    .map(move |plugin_name| {
                        package_dependencies
                            .iter()
                            .find(|id| id.name == *plugin_name)
                            .map(|id| {
                                self.packages.get(id).expect("workspace resolve packages must include all dependency graph nodes").clone()
                            })
                            .filter(|package| package.is_cairo_plugin())
                            .map(Ok)
                            .unwrap_or_else(|| {
                                bail!(
                                "package `{}` cannot re-export cairo plugin `{plugin_name}` which is not a dependency of `{}`",
                                dependency.id.name, dependency.id.name)
                            })
                    })

            })
            .collect_vec();

        dependencies
            .into_iter()
            .map(Ok)
            .chain(re_exported)
            .collect::<Result<Vec<Package>>>()
    }

    /// Get all dependencies with allowed prebuilt macros for a given package.
    pub fn allowed_prebuilt(
        &self,
        package: Package,
        target_kind: &TargetKind,
    ) -> Result<AllowedPrebuiltFilter> {
        let metadata = package.scarb_tool_metadata()?;
        let allowed = metadata.allow_prebuilt_plugins.unwrap_or_default();
        let allowed = allowed
            .into_iter()
            .filter_map(|name| PackageName::try_new(name).ok())
            .map(|name| name.to_smol_str())
            .collect();
        let allowed =
            self.resolve
                .filter_subtrees(target_kind, allowed, |package_id: PackageId| {
                    package_id.name.to_smol_str()
                });
        let allowed_prebuilds = AllowedPrebuiltFilter::new(
            allowed
                .into_iter()
                .map(PackageName::new)
                .collect::<HashSet<_>>(),
        );
        Ok(allowed_prebuilds)
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
    ws.config()
        .tokio_handle()
        .block_on(TryFutureExt::into_future(async {
            let mut patch_map = PatchMap::new();

            for (source, patches) in ws.patch() {
                patch_map.insert(source.clone(), patches.clone());
            }

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
                        .name(PackageName::EXECUTABLE)
                        .version_req(version_req.clone())
                        .source_id(SourceId::for_std())
                        .build(),
                    ManifestDependency::builder()
                        .kind(DepKind::Target(TargetKind::TEST))
                        .name(PackageName::TEST_PLUGIN)
                        .version_req(version_req.clone())
                        .source_id(SourceId::for_std())
                        .build(),
                    ManifestDependency::builder()
                        .kind(DepKind::Target(TargetKind::LIB))
                        .name(PackageName::CAIRO_RUN_PLUGIN)
                        .version_req(version_req.clone())
                        .source_id(SourceId::for_std())
                        .build(),
                    ManifestDependency::builder()
                        .kind(DepKind::Target(TargetKind::TEST))
                        .name(PackageName::TEST_ASSERTS_PLUGIN)
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

            let members_summaries = ws
                .members()
                .map(|pkg| pkg.manifest.summary.clone())
                .collect::<Vec<_>>();

            let (lockfile, yanked_whitelist) = if opts.update {
                (Lockfile::new([]), HashSet::new())
            } else {
                let lockfile = read_lockfile(ws)?;
                let yanked_whitelist = lockfile.create_yanked_whitelist();
                (lockfile, yanked_whitelist)
            };

            let source_map = SourceMap::preloaded(ws.members(), ws.config(), yanked_whitelist);
            let cached = RegistryCache::new(&source_map);
            let patched = RegistryPatcher::new(&cached, &patch_map);

            let resolve =
                resolver::resolve(&members_summaries, &patched, &patch_map, lockfile).await?;

            let lockfile = tokio::spawn(write_lockfile(
                Lockfile::from_resolve(&resolve),
                ws.lockfile_path(),
            ));

            patch_map.warn_unused(ws.config().ui());

            let packages = collect_packages_from_resolve_graph(&resolve, &patched).await?;

            packages
                .values()
                .filter(|p| p.is_cairo_plugin())
                .map(|p| fetch_cairo_plugin(p, ws))
                .collect::<Result<Vec<()>>>()?;

            lockfile.await??;

            Ok(WorkspaceResolve { resolve, packages })
        }))
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
    let mut response_stream = stream::iter(resolve.package_ids())
        .map(|package_id| registry.download(package_id).boxed_local())
        // Note: This value limits the number of concurrent downloads.
        // It's chosen rather arbitrarily, so may be subject to more tuning in the future.
        .buffer_unordered(16);
    while let Some(package) = response_stream.next().await {
        let package = package?;
        packages.insert(package.id, package);
    }
    Ok(packages)
}

#[derive(Debug, Default)]
pub struct AllowedPrebuiltFilter(HashSet<PackageName>);

impl AllowedPrebuiltFilter {
    pub fn new(allowed: HashSet<PackageName>) -> Self {
        Self(allowed)
    }

    pub fn check(&self, package: &Package) -> bool {
        self.0.contains(&package.id.name)
    }
}

pub struct CompilationUnitsOpts {
    /// Skip cairo version requirements check.
    ///
    /// This will ignore requirements defined in `cairo-version` field in package manifest.
    pub ignore_cairo_version: bool,
    /// Load prebuilt macros for procedural macros.
    ///
    /// Disabling this flag is useful if the generated compilation units will not be used to compile
    /// users project. For example, when generating units for scarb-metadata.
    /// Note, even if `true`, only macros allowed in package manifest will be loaded.
    pub load_prebuilt_macros: bool,
}

#[tracing::instrument(skip_all, level = "debug")]
pub fn generate_compilation_units(
    resolve: &WorkspaceResolve,
    enabled_features: &FeaturesOpts,
    ws: &Workspace<'_>,
    opts: CompilationUnitsOpts,
) -> Result<Vec<CompilationUnit>> {
    let mut cairo_units: Vec<CairoCompilationUnit> = Vec::with_capacity(ws.members().size_hint().0);
    let members = ws
        .members()
        .filter(|member| !member.is_cairo_plugin())
        .collect_vec();
    validate_features(&members, enabled_features)?;
    for member in members {
        cairo_units.extend(generate_cairo_compilation_units(
            &member,
            resolve,
            enabled_features,
            opts.ignore_cairo_version,
            ws,
        )?);
    }

    let proc_macro_units = if ws.config().proc_macro_repository().load_proc_macros() {
        cairo_units
            .iter()
            .flat_map(|unit| unit.cairo_plugins.clone())
            .filter(|plugin| !plugin.builtin)
            .map(|plugin| (plugin.package.clone(), plugin.prebuilt_allowed))
            .chain(
                ws.members()
                    .filter(|member| member.is_cairo_plugin())
                    .map(|member| (member, false)),
            )
            // In case some prebuilt macro is allowed for one workspace member and disallowed for
            // the other, we need to set `prebuilt_allowed` to `false` for that macro package, so that
            // it is compiled with Cargo.
            // This works by placing packages with `prebuilt_allowed` set to `false` first
            // in the iterator and relying on stability of `unique_by` method to skip duplicates with
            // allowed prebuilt macros (retaining once with disabled).
            .sorted_by_key(|(_, prebuilt_allowed)| if *prebuilt_allowed { 1 } else { 0 })
            .unique_by(|(plugin, _)| plugin.id)
            .map(|(plugin, prebuilt_allowed)| {
                Ok((
                    plugin.id,
                    generate_cairo_plugin_compilation_units(
                        &plugin,
                        opts.load_prebuilt_macros && prebuilt_allowed,
                    )?,
                ))
            })
            .collect::<Result<HashMap<PackageId, ProcMacroCompilationUnit>>>()?
    } else {
        HashMap::new()
    };

    let units = cairo_units
        .into_iter()
        .map(|mut unit| {
            for plugin in &mut unit.cairo_plugins {
                if let Some(proc_macro_unit) = proc_macro_units.get(&plugin.package.id) {
                    plugin.prebuilt = plugin
                        // We check if prebuilt is allowed for this compilation unit, as it might
                        // be disabled for some workspace members even if other members allow it.
                        .prebuilt_allowed
                        .then_some(proc_macro_unit.prebuilt.clone())
                        .flatten();
                }
            }
            unit
        })
        .map(CompilationUnit::Cairo)
        .collect_vec();
    let units: Vec<CompilationUnit> = units
        .into_iter()
        .chain(
            proc_macro_units
                .into_values()
                // Sort for stability.
                .sorted_by_key(|unit| unit.main_package_id)
                .map(CompilationUnit::ProcMacro),
        )
        .collect();

    assert!(
        units.iter().map(CompilationUnit::id).all_unique(),
        "All generated compilation units must have unique IDs."
    );

    Ok(units)
}

pub fn validate_features(members: &[Package], enabled_features: &FeaturesOpts) -> Result<()> {
    // Check if any member has features defined.
    if let FeaturesSelector::Features(features) = &enabled_features.features {
        for feature in features {
            if !members
                .iter()
                .any(|member| member.manifest.features.contains_key(feature))
            {
                bail!(
                    "none of the selected packages contains `{}` feature\n\
                    note: to use features, you need to define [features] section in Scarb.toml",
                    feature
                );
            }
        }
    }
    Ok(())
}

fn generate_cairo_compilation_units(
    member: &Package,
    resolve: &WorkspaceResolve,
    enabled_features: &FeaturesOpts,
    ignore_cairo_version: bool,
    ws: &Workspace<'_>,
) -> Result<Vec<CairoCompilationUnit>> {
    let profile = ws.current_profile()?;
    let mut solution = PackageSolutionCollector::new(member, resolve, ws);
    let grouped = member
        .manifest
        .targets
        .iter()
        .filter(|target| target.group_id.is_some())
        .chunk_by(|target| target.group_id.clone())
        .into_iter()
        .map(|(group_id, group)| (group_id, group.collect_vec()))
        .sorted_by_key(|(_, group)| group[0].kind.clone())
        .map(|(_group_id, group)| {
            let group = group.into_iter().cloned().collect_vec();
            cairo_compilation_unit_for_target(
                group,
                member,
                profile.clone(),
                enabled_features,
                ignore_cairo_version,
                &mut solution,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let result = member
        .manifest
        .targets
        .iter()
        .filter(|target| target.group_id.is_none())
        .map(|member_target| {
            cairo_compilation_unit_for_target(
                vec![member_target.clone()],
                member,
                profile.clone(),
                enabled_features,
                ignore_cairo_version,
                &mut solution,
            )
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .chain(grouped)
        .collect();
    solution.show_warnings();
    Ok(result)
}

fn cairo_compilation_unit_for_target(
    member_targets: Vec<Target>,
    member: &Package,
    profile: Profile,
    enabled_features: &FeaturesOpts,
    ignore_cairo_version: bool,
    solution: &mut PackageSolutionCollector<'_>,
) -> Result<CairoCompilationUnit> {
    let member_target = member_targets.first().cloned().unwrap();

    let selected_features: Vec<FeatureName> = match &enabled_features.features {
        FeaturesSelector::AllFeatures => member.manifest.features.all().cloned().collect(),
        FeaturesSelector::Features(features) => features.clone(),
    };
    let selected_features = member
        .manifest
        .features
        .select(&selected_features, !enabled_features.no_default_features);

    solution.collect(
        &member_target.kind,
        &selected_features.enabled(),
        ignore_cairo_version,
    )?;
    let packages = solution.packages().unwrap();
    let cairo_plugins = solution.cairo_plugins().unwrap();
    let features_for_deps = solution.features_for_deps().unwrap();

    let unit_cfg_set = build_cfg_set(&member_target, &member.manifest.compiler_config);
    let no_test_cfg_set: CfgSet = unit_cfg_set
        .iter()
        .filter(|cfg| **cfg != Cfg::name("test"))
        .cloned()
        .collect();

    let props: TestTargetProps = member_target.props()?;
    let is_integration_test = props.test_type == TestTargetType::Integration;
    let name = member_target
        .group_id
        .clone()
        .unwrap_or(member_target.name.clone());
    let test_package_id = member.id.for_test_target(name);

    let mut components: Vec<CompilationUnitComponent> = packages
        .iter()
        .cloned()
        .map(|package| {
            // If this is this compilation's unit main package, then use the target we are
            // building. Otherwise, assume library target for all dependency packages,
            // because that's what it is for.
            let targets = if package.id == member.id {
                if member_targets.len() == 1 && member_targets[0].group_id.is_none() {
                    ComponentTarget::new_single(member_targets[0].clone())
                } else {
                    ComponentTarget::try_new_group(member_targets.clone())?
                }
            } else {
                // We can safely unwrap here, because compilation unit generator ensures
                // that all dependencies have library target.
                ComponentTarget::new_single(package.fetch_target(&TargetKind::LIB).unwrap().clone())
            };

            // For integration tests target, rewrite package with prefixed name.
            // This allows integration test code to reference main package as dependency.
            let package_id_rewritten = package.id == member.id && is_integration_test;
            let package = if package_id_rewritten {
                Package::new(
                    test_package_id,
                    package.manifest_path().to_path_buf(),
                    package.manifest.clone(),
                )
            } else {
                package
            };

            let dep_features = features_for_deps
                .get(&package.id)
                .cloned()
                .unwrap_or_default();
            let cfg_set = {
                if package.id == member.id || package_id_rewritten {
                    // This is the main package.
                    CfgSetBuilder::from_manifest_features(
                        &package.manifest.features,
                        enabled_features,
                    )
                    .with_cfg_set(unit_cfg_set.clone())
                    .extend_features(dep_features)
                    .into()
                } else {
                    CfgSetBuilder::new()
                        .with_cfg_set(no_test_cfg_set.clone())
                        .extend_features(dep_features)
                        .into()
                }
            };
            let cfg_set = (cfg_set != unit_cfg_set).then_some(cfg_set);
            CompilationUnitComponent::try_new(package, targets, cfg_set)
        })
        .collect::<Result<_>>()?;

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

        let dep_features = features_for_deps
            .get(&member.id)
            .cloned()
            .unwrap_or_default();
        let cfg_set =
            CfgSetBuilder::from_manifest_features(&member.manifest.features, enabled_features)
                .with_cfg_set(no_test_cfg_set)
                .extend_features(dep_features)
                .into();
        let cfg_set = (cfg_set != unit_cfg_set).then_some(cfg_set);
        // Add `lib` target for tested package, to be available as dependency.
        components.push(CompilationUnitComponent::try_new(
            member.clone(),
            ComponentTarget::new_single(target),
            cfg_set,
        )?);

        // Set test package as main package for this compilation unit.
        test_package_id
    } else {
        member.id
    };

    // Collect dependencies for the components.
    let member_component = components
        .iter()
        .find(|component| component.package.id == member.id)
        .unwrap();
    let mut test_package_deps =
        solution.component_dependencies(member_component, &components, member.id)?;
    if is_integration_test {
        test_package_deps.push(CompilationUnitDependency::Library(
            member_component.id.clone(),
        ));
    }

    let dependencies_for_components: Vec<_> = components
        .iter()
        .map(|component| {
            Ok(if component.package.id == test_package_id {
                test_package_deps.clone()
            } else {
                solution.component_dependencies(component, &components, member.id)?
            })
        })
        .collect::<Result<Vec<_>>>()?;

    for (component, dependencies) in zip(&mut components, dependencies_for_components) {
        component.dependencies = dependencies;
    }

    Ok(CairoCompilationUnit {
        main_package_id,
        components,
        cairo_plugins: cairo_plugins.clone(),
        profile: profile.clone(),
        compiler_config: member.manifest.compiler_config.clone(),
        cfg_set: unit_cfg_set,
    })
}

struct CfgSetBuilder {
    features: HashSet<FeatureName>,
    cfg_set: CfgSet,
}

impl CfgSetBuilder {
    pub fn new() -> Self {
        Self {
            cfg_set: Default::default(),
            features: Default::default(),
        }
    }

    pub fn from_manifest_features(
        features_manifest: &FeaturesDefinition,
        enabled_features: &FeaturesOpts,
    ) -> Self {
        let selected_features: Vec<FeatureName> = match &enabled_features.features {
            FeaturesSelector::AllFeatures => features_manifest.all().cloned().collect(),
            FeaturesSelector::Features(features) => features.clone(),
        };
        let selected_features =
            features_manifest.select(&selected_features, !enabled_features.no_default_features);
        let features = selected_features.enabled();
        Self::new().extend_features(features)
    }

    pub fn with_cfg_set(mut self, cfg_set: CfgSet) -> Self {
        self.cfg_set = cfg_set;
        self
    }

    pub fn extend_features(mut self, features: HashSet<FeatureName>) -> Self {
        self.features.extend(features);
        self
    }

    pub fn build(self) -> CfgSet {
        let mut cfg_set = self.cfg_set;
        for feature in self.features {
            cfg_set.insert(Cfg::kv("feature", feature.to_string()));
        }
        cfg_set
    }
}

impl From<CfgSetBuilder> for CfgSet {
    fn from(builder: CfgSetBuilder) -> Self {
        builder.build()
    }
}

pub struct PackageSolutionCollector<'a> {
    member: &'a Package,
    resolve: &'a WorkspaceResolve,
    ws: &'a Workspace<'a>,
    warnings: HashSet<String>,
    collected: Option<CollectedResolution>,
}

impl<'a> PackageSolutionCollector<'a> {
    pub fn new(member: &'a Package, resolve: &'a WorkspaceResolve, ws: &'a Workspace<'a>) -> Self {
        Self {
            member,
            resolve,
            ws,
            warnings: HashSet::new(),
            collected: Default::default(),
        }
    }

    pub fn collect(
        &mut self,
        target_kind: &TargetKind,
        selected_features: &HashSet<FeatureName>,
        ignore_cairo_version: bool,
    ) -> Result<()> {
        // Do not traverse graph for each target of the same kind.
        if !self
            .collected
            .as_ref()
            .map(|collected| &collected.target_kind == target_kind)
            .unwrap_or(false)
        {
            let collected =
                self.pull_from_graph(target_kind, selected_features, ignore_cairo_version)?;
            self.collected = Some(collected);
        }
        Ok(())
    }

    fn pull_from_graph(
        &mut self,
        target_kind: &TargetKind,
        selected_features: &HashSet<FeatureName>,
        ignore_cairo_version: bool,
    ) -> Result<CollectedResolution> {
        let allowed_prebuilds = self
            .resolve
            .allowed_prebuilt(self.member.clone(), target_kind)?;
        let features_for_deps =
            self.resolve
                .features_unification(self.member.id, selected_features, target_kind)?;
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

        check_cairo_version_compatibility(
            &packages,
            self.ws,
            &mut self.warnings,
            ignore_cairo_version,
        )?;

        // Print warnings for dependencies that are not usable.
        let other = classes.remove(&PackageClass::Other).unwrap_or_default();
        for pkg in other {
            self.warnings.insert(format!(
                "{} ignoring invalid dependency `{}` which is missing a lib or cairo-plugin target",
                self.member.id, pkg.id.name
            ));
        }

        let cairo_plugins = cairo_plugins
            .into_iter()
            .map(|package| {
                let prebuilt_allowed = allowed_prebuilds.check(&package);
                // We can safely unwrap as all packages with `PackageClass::CairoPlugin` must define plugin target.
                let target = package.target(&TargetKind::CAIRO_PLUGIN).unwrap();
                let props: CairoPluginProps = target.props()?;
                Ok(CompilationUnitCairoPlugin::builder()
                    .component_dependency_id(CompilationUnitComponentId {
                        package_id: package.id,
                    })
                    .package(package)
                    .builtin(props.builtin)
                    .prebuilt_allowed(prebuilt_allowed)
                    .prebuilt(None)
                    .cached_shared_lib_path(Default::default())
                    .build())
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(CollectedResolution {
            packages,
            cairo_plugins,
            features_for_deps,
            target_kind: target_kind.clone(),
        })
    }

    pub fn component_dependencies(
        &self,
        component: &CompilationUnitComponent,
        components: &[CompilationUnitComponent],
        main_package_id: PackageId,
    ) -> Result<Vec<CompilationUnitDependency>> {
        let package_id = component.id.package_id;
        let component_target_kind = self.target_kind().unwrap();

        // Those are direct dependencies of the component.
        let dependencies_packages = self.resolve.package_dependencies(
            package_id,
            component_target_kind,
            main_package_id,
        )?;

        // We iterate over all the compilation unit components to get dependency's version.
        let mut dependencies: HashSet<_> = components
            .iter()
            .filter(|component_as_dependency| {
                dependencies_packages.iter().any(|dependency_summary| {
                    dependency_summary.id == component_as_dependency.package.id
                })
            })
            .map(|component| CompilationUnitDependency::Library(component.id.clone()))
            .collect();

        // Adds itself to dependencies
        let is_integration_test = if component.targets.target_kind().is_test() {
            let props: Option<TestTargetProps> = component.targets.target_props().ok();
            props
                .map(|props| props.test_type == TestTargetType::Integration)
                .unwrap_or_default()
        } else {
            false
        };
        if !is_integration_test {
            dependencies.insert(CompilationUnitDependency::Library(component.id.clone()));
        }

        let plugin_dependencies = dependencies_packages
            .iter()
            .filter(|package| package.is_cairo_plugin())
            .map(|package| {
                CompilationUnitDependency::Plugin(CompilationUnitComponentId {
                    package_id: package.id,
                })
            })
            .collect::<Vec<_>>();

        dependencies.extend(plugin_dependencies);
        Ok(dependencies.into_iter().collect())
    }

    pub fn show_warnings(self) {
        for warning in self.warnings {
            self.ws.config().ui().warn(warning);
        }
    }

    fn target_kind(&self) -> Option<&TargetKind> {
        self.collected
            .as_ref()
            .map(|collected| &collected.target_kind)
    }

    pub fn packages(&self) -> Option<&Vec<Package>> {
        self.collected.as_ref().map(|collected| &collected.packages)
    }
    pub fn cairo_plugins(&self) -> Option<&Vec<CompilationUnitCairoPlugin>> {
        self.collected
            .as_ref()
            .map(|collected| &collected.cairo_plugins)
    }

    pub fn features_for_deps(&self) -> Option<&HashMap<PackageId, HashSet<FeatureName>>> {
        self.collected
            .as_ref()
            .map(|collected| &collected.features_for_deps)
    }
}

struct CollectedResolution {
    pub packages: Vec<Package>,
    pub cairo_plugins: Vec<CompilationUnitCairoPlugin>,
    pub target_kind: TargetKind,
    pub features_for_deps: HashMap<PackageId, HashSet<FeatureName>>,
}

/// Build a set of `cfg` items to enable while building the compilation unit.
fn build_cfg_set(target: &Target, compiler_config: &ManifestCompilerConfig) -> CfgSet {
    let mut cfg = CfgSet::from_iter([Cfg::kv("target", target.kind.clone())]);
    if target.is_test() {
        cfg.insert(Cfg::name("test"));
    }
    if !compiler_config.enable_gas {
        cfg.insert(Cfg::kv("gas", "disabled"));
    }
    cfg
}

fn check_cairo_version_compatibility(
    packages: &[Package],
    ws: &Workspace<'_>,
    warnings: &mut HashSet<String>,
    ignore_mismatch: bool,
) -> Result<()> {
    let current_version = crate::version::get().cairo.version.to_version().unwrap();
    let matching_version = packages
        .iter()
        .all(|pkg| match &pkg.manifest.metadata.cairo_version {
            Some(package_version) if !package_version.matches(&current_version) => {
                let msg = formatdoc!(
                    r"
                    the required Cairo version of package {} is not compatible with current version
                    Cairo version required: {}
                    Cairo version of Scarb: {}
                    ",
                    pkg.id.name,
                    package_version,
                    current_version
                );
                if ignore_mismatch {
                    warnings.insert(msg);
                } else {
                    ws.config().ui().error(msg);
                }
                false
            }
            _ => true,
        });
    if !matching_version && !ignore_mismatch {
        bail!(formatdoc! { r"
                the required Cairo version of each package must match the current Cairo version
                help: pass `--ignore-cairo-version` to ignore Cairo version mismatch
            "})
    }
    Ok(())
}

pub fn generate_cairo_plugin_compilation_units(
    member: &Package,
    // Whether loading a prebuilt library is both allowed and requested.
    load_prebuilt: bool,
) -> Result<ProcMacroCompilationUnit> {
    let load_prebuilt = load_prebuilt && member.prebuilt_lib_path().is_some();
    let prebuilt = load_prebuilt
        .then_some(
            ProcMacroInstance::try_load_prebuilt(member.clone())
                // Note we ignore loading errors here.
                // If the prebuilt library is corrupted, it will be later compiled with Cargo,
                // like there was no prebuilt defined.
                .ok()
                .map(Arc::new),
        )
        .flatten();
    let components = vec![CompilationUnitComponent::try_new(
        member.clone(),
        ComponentTarget::new_single(
            member
                .fetch_target(&TargetKind::CAIRO_PLUGIN)
                .cloned()
                // Safe to unwrap, as member.is_cairo_plugin() has been ensured before.
                .expect("main component of procedural macro must define `cairo-plugin` target"),
        ),
        None,
    )?];
    Ok(ProcMacroCompilationUnit {
        main_package_id: member.id,
        compiler_config: serde_json::Value::Null,
        components,
        prebuilt,
    })
}

/// Generate package ids associated with test compilation units for the given packages.
/// This function will return input list along with generated test package ids.
pub fn get_test_package_ids(packages: Vec<PackageId>, ws: &Workspace<'_>) -> Vec<PackageId> {
    packages
        .into_iter()
        .flat_map(|package_id| {
            let Some(package) = ws.members().find(|p| p.id == package_id) else {
                return Vec::new();
            };
            let mut result: Vec<PackageId> = package
                .manifest
                .targets
                .iter()
                .filter(|t| t.is_test())
                .map(|t| {
                    package
                        .id
                        .for_test_target(t.group_id.clone().unwrap_or(t.name.clone()))
                })
                .collect();
            result.push(package_id);
            result
        })
        .collect::<Vec<PackageId>>()
}
