use crate::compiler::db::{
    ScarbDatabase, append_lint_plugin, apply_plugins, build_project_config,
    build_scarb_root_database, has_plugin, inject_virtual_wrapper_lib, is_starknet_plugin,
};
use crate::compiler::helpers::{build_compiler_config, collect_main_crate_ids};
use crate::compiler::incremental::IncrementalContext;
use crate::compiler::plugin::collection::PluginsForComponents;
use crate::compiler::plugin::proc_macro;
use crate::compiler::plugin::proc_macro::ProcMacroHostPlugin;
use crate::compiler::{CairoCompilationUnit, CompilationUnit, CompilationUnitAttributes};
use crate::core::{
    FeatureName, PackageId, PackageName, TargetKind, Utf8PathWorkspaceExt, Workspace,
};
use crate::flock::Filesystem;
use crate::internal::fsx;
use crate::internal::offloader::Offloader;
use crate::ops;
use crate::ops::{CompilationUnitsOpts, get_test_package_ids, validate_features};
use anyhow::{Context, Error, Result, anyhow, bail, ensure};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::DiagnosticsError;
use cairo_lang_compiler::project::{
    get_main_crate_ids_from_project, update_crate_roots_from_project_config,
};
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_semantic::plugin::PluginSuite;
use camino::Utf8PathBuf;
use indoc::formatdoc;
use itertools::Itertools;
use salsa::Database;
use scarb_ui::HumanDuration;
use scarb_ui::args::FeaturesSpec;
use scarb_ui::components::Status;
use smol_str::{SmolStr, ToSmolStr};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::thread;
use tracing::{trace, trace_span};

#[derive(Debug, Clone)]
pub enum FeaturesSelector {
    Features(Vec<FeatureName>),
    AllFeatures,
}

#[derive(Debug, Clone)]
pub struct FeaturesOpts {
    pub features: FeaturesSelector,
    pub no_default_features: bool,
}

impl TryFrom<FeaturesSpec> for FeaturesOpts {
    type Error = Error;
    fn try_from(spec: FeaturesSpec) -> Result<Self> {
        Ok(Self {
            features: if spec.all_features {
                FeaturesSelector::AllFeatures
            } else {
                FeaturesSelector::Features(
                    spec.features
                        .into_iter()
                        .filter(|f| !f.is_empty())
                        .map(FeatureName::try_from)
                        .try_collect()?,
                )
            },
            no_default_features: spec.no_default_features,
        })
    }
}

#[derive(Debug)]
pub struct CompileOpts {
    pub include_target_kinds: Vec<TargetKind>,
    pub exclude_target_kinds: Vec<TargetKind>,
    pub include_target_names: Vec<SmolStr>,
    pub features: FeaturesOpts,
    pub ignore_cairo_version: bool,
}

impl CompileOpts {
    pub fn try_new(
        features: FeaturesSpec,
        ignore_cairo_version: bool,
        test: bool,
        target_names: Vec<String>,
        target_kinds: Vec<String>,
    ) -> Result<Self> {
        let (include_targets, exclude_targets): (Vec<TargetKind>, Vec<TargetKind>) = if test {
            (vec![TargetKind::TEST.clone()], Vec::new())
        } else {
            (Vec::new(), vec![TargetKind::TEST.clone()])
        };
        let include_targets = if !target_kinds.is_empty() {
            target_kinds
                .into_iter()
                .map(TargetKind::try_new)
                .collect::<Result<Vec<TargetKind>>>()?
        } else {
            include_targets
        };
        Ok(Self {
            include_target_kinds: include_targets,
            exclude_target_kinds: exclude_targets,
            include_target_names: target_names
                .into_iter()
                .map(|v| v.to_smolstr())
                .collect_vec(),
            features: features.try_into()?,
            ignore_cairo_version,
        })
    }
}

#[tracing::instrument(skip_all, level = "debug")]
pub fn compile(packages: Vec<PackageId>, opts: CompileOpts, ws: &Workspace<'_>) -> Result<()> {
    process(packages, opts, ws, compile_units, None)
}

#[tracing::instrument(skip_all, level = "debug")]
pub fn check(packages: Vec<PackageId>, opts: CompileOpts, ws: &Workspace<'_>) -> Result<()> {
    process(packages, opts, ws, check_units, Some("checking"))
}

#[tracing::instrument(skip_all, level = "debug")]
fn process<F>(
    packages: Vec<PackageId>,
    opts: CompileOpts,
    ws: &Workspace<'_>,
    mut operation: F,
    operation_type: Option<&str>,
) -> Result<()>
where
    F: FnMut(Vec<CompilationUnit>, &[PackageId], &Workspace<'_>) -> Result<()>,
{
    let resolve = ops::resolve_workspace(ws)?;
    let packages_to_process = ws
        .members()
        .filter(|p| packages.contains(&p.id))
        .collect_vec();
    validate_features(&packages_to_process, &opts.features)?;

    // Run prebuild scripts for all packages being compiled.
    for package in &packages_to_process {
        ops::execute_magic_script_if_exists("build", package, ws)?;
    }

    // Add test compilation units to build
    let packages = get_test_package_ids(packages, ws);
    let compilation_units = ops::generate_compilation_units(
        &resolve,
        &opts.features,
        ws,
        CompilationUnitsOpts {
            ignore_cairo_version: opts.ignore_cairo_version,
            load_prebuilt_macros: ws.config().load_prebuilt_proc_macros(),
        },
    )?
    .into_iter()
    .filter(|cu| {
        let is_excluded = opts
            .exclude_target_kinds
            .contains(&cu.main_component().target_kind());
        let is_included = opts.include_target_kinds.is_empty()
            || opts
                .include_target_kinds
                .contains(&cu.main_component().target_kind());
        let is_included = is_included
            && (opts.include_target_names.is_empty()
                || cu
                    .main_component()
                    .targets
                    .targets()
                    .iter()
                    .any(|t| opts.include_target_names.contains(&t.name)));
        let is_selected = packages.contains(&cu.main_package_id());
        let is_cairo_plugin = matches!(cu, CompilationUnit::ProcMacro(_));
        is_cairo_plugin || (is_selected && is_included && !is_excluded)
    })
    // Proc macro compilations are processed first, as Cairo compilation units may depend on them.
    .sorted_by_key(|cu| {
        if matches!(cu, CompilationUnit::ProcMacro(_)) {
            0
        } else {
            1
        }
    })
    .collect::<Vec<_>>();

    operation(compilation_units, &packages, ws)?;

    let elapsed_time = HumanDuration(ws.config().elapsed_time());
    let profile = ws.current_profile()?;
    let formatted_message = match operation_type {
        Some(op) => format!("{op} `{profile}` profile target(s) in {elapsed_time}"),
        None => format!("`{profile}` profile target(s) in {elapsed_time}"),
    };
    ws.config()
        .ui()
        .print(Status::new("Finished", &formatted_message));

    Ok(())
}

pub fn compile_units(
    units: Vec<CompilationUnit>,
    required_packages: &[PackageId],
    ws: &Workspace<'_>,
) -> Result<()> {
    let required_plugins = plugins_required_for_units(&units);
    let grouped_units: BTreeMap<PackageId, Vec<CompilationUnit>> = {
        let mut grouped: BTreeMap<PackageId, Vec<CompilationUnit>> = BTreeMap::new();
        for unit in units {
            grouped.entry(unit.main_package_id()).or_default().push(unit);
        }
        grouped
    };

    for (package_id, units) in grouped_units {

        if let Some(pkg) = ws.package(&package_id) {
            // Build RootDatabase configured according to the package's compiler config.
            let mut db = {

                let mut b = RootDatabase::builder();
                if !pkg.manifest.compiler_config.enable_gas {
                    b.skip_auto_withdraw_gas();
                }
                if pkg.manifest.compiler_config.panic_backtrace {
                    b.with_panic_backtrace();
                }
                if pkg.manifest.compiler_config.unsafe_panic {
                    b.with_unsafe_panic();
                }
                b.build()?
            };

            for unit in units {
                // We can skip compiling proc macros that are not used by Cairo compilation units,
                // unless they were explicitly requested in required_packages.
                if is_unused_proc_macro(&unit, &required_plugins, required_packages) {
                    continue;
                }
                compile_unit(unit, ws, Some(&mut db))?;
            }
        } else {
            // No package found for this package_id; build a default RootDatabase.
            for unit in units {
                // We can skip compiling proc macros that are not used by Cairo compilation units,
                // unless they were explicitly requested in required_packages.
                if is_unused_proc_macro(&unit, &required_plugins, required_packages) {
                    continue;
                }
                let mut db = {
                    let mut b = RootDatabase::builder();
                    b.build()?
                };
                compile_unit(unit, ws, Some(&mut db))?;
            }
        }
    }
    Ok(())
}

#[tracing::instrument(level = "debug", skip_all)]
fn compile_cairo_unit_inner(
    unit: &CairoCompilationUnit,
    db: &mut RootDatabase,
    ws: &Workspace<'_>,
    additional_plugins: Vec<PluginSuite>,
) -> Result<()> {
    ws.config()
        .ui()
        .print(Status::new("Compiling", &unit.name()));

    let project_config = build_project_config(unit)?;
    update_crate_roots_from_project_config(db, &project_config);
    db.use_cfg(&unit.cfg_set);

    let PluginsForComponents {
        mut plugins,
        proc_macros,
    } = {
        PluginsForComponents::collect(ws, unit)?
    };
    append_lint_plugin(plugins.get_mut(&unit.main_component().id).unwrap());

    let main_component_suite = plugins
        .get_mut(&unit.main_component().id)
        .expect("should be able to retrieve plugins for main component");

    for additional_suite in additional_plugins.iter() {
        main_component_suite.add(additional_suite.clone());
    }
    apply_plugins(db, plugins);
    inject_virtual_wrapper_lib(db, unit)?;

    let proc_macros: Vec<ProcMacroHostPlugin> = {
    proc_macros
        .into_values()
        .flat_map(|hosts| hosts.into_iter())
        .collect()
    };

    let package_name = unit.main_package_id().name.clone();
    check_starknet_dependency(unit, ws, db, &package_name);
    let assets = collect_assets(unit)?;

    thread::scope(|s| {
        let offloader = Offloader::new(s, ws);
        let target_dir = unit.target_dir(ws);

        let result = ws
                .config()
                .compilers()
                .compile(unit.clone(), &offloader, db, ws);

        let main_crate_ids = get_main_crate_ids_from_project(db, &project_config);
        for plugin in proc_macros {
            plugin
                .post_process(db, &main_crate_ids)
                .context("procedural macro post processing callback failed")?;
        }

            offloader.join()?;

        if result.is_ok() {
            copy_assets(assets, &target_dir)?;
        }
        result
    })
}

/// Run compiler in a new thread.
/// The stack size of created threads can be altered with `RUST_MIN_STACK` env variable.
pub fn compile_unit(
    unit: CompilationUnit,
    ws: &Workspace<'_>,
    db: Option<&mut RootDatabase>,
) -> Result<()> {
    thread::scope(|s| {
        thread::Builder::new()
            .name(format!("scarb compile {}", unit.id()))
            .spawn_scoped(s, || compile_unit_inner(unit, ws, db))
            .expect("Failed to spawn compiler thread.")
            .join()
            .expect("Compiler thread has panicked.")
    })
}

#[tracing::instrument(skip_all, level = "trace")]
fn compile_unit_inner(
    unit: CompilationUnit,
    ws: &Workspace<'_>,
    db: Option<&mut RootDatabase>,
) -> Result<()> {
    let package_name = unit.main_package_id().name.clone();

    let result = match unit {
        CompilationUnit::ProcMacro(unit) => {
            if unit.prebuilt.is_some() {
                Ok(())
            } else {
                ws.config()
                    .ui()
                    .print(Status::new("Compiling", &unit.name()));
                proc_macro::compile_unit(unit, ws)
            }
        }
        CompilationUnit::Cairo(unit) => {
            let db = db.expect("db must be provided for Cairo compilation units");
            compile_cairo_unit_inner(&unit, db, ws, Default::default())
        }
    };

    result.map_err(|err| {
        if !suppress_error(&err) {
            ws.config().ui().anyhow(&err);
        }

        anyhow!("could not compile `{package_name}` due to previous error")
    })
}

fn check_units(
    units: Vec<CompilationUnit>,
    _required_packages: &[PackageId],
    ws: &Workspace<'_>,
) -> Result<()> {
    let required_plugins = plugins_required_for_units(&units);

    for unit in units {
        if matches!(unit, CompilationUnit::ProcMacro(_))
            && required_plugins.contains(&unit.main_package_id())
        {
            // We compile proc macros that will be used by latter Cairo CUs.
            // Note: this only works, because `process` maintains the order of units.
            compile_unit(unit, ws, None)?;
        } else {
            check_unit(unit, ws)?;
        }
    }
    Ok(())
}

fn check_unit(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let package_name = unit.main_package_id().name.clone();

    ws.config()
        .ui()
        .print(Status::new("Checking", &unit.name()));

    let result = match unit {
        CompilationUnit::ProcMacro(unit) => proc_macro::check_unit(unit, ws),
        CompilationUnit::Cairo(unit) => {
            let ScarbDatabase { db, .. } =
                build_scarb_root_database(&unit, ws, Default::default())?;
            let main_crate_ids = collect_main_crate_ids(&unit, &db);
            check_starknet_dependency(&unit, ws, &db, &package_name);
            let mut compiler_config = build_compiler_config(
                &db,
                &unit,
                &main_crate_ids,
                &IncrementalContext::Disabled,
                ws,
            );
            let result = compiler_config
                .diagnostics_reporter
                .ensure(&db)
                .map_err(|err| err.into());
            let _ = main_crate_ids;
            drop(compiler_config);
            let span = trace_span!("drop_db");
            {
                let _guard = span.enter();
                drop(db);
            }
            result
        }
    };

    result.map_err(|err| {
        if !suppress_error(&err) {
            ws.config().ui().anyhow(&err);
        }

        anyhow!("could not check `{package_name}` due to previous error")
    })?;

    Ok(())
}

fn check_starknet_dependency(
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
    db: &dyn Database,
    package_name: &PackageName,
) {
    // NOTE: This is a special case that can be hit frequently by newcomers. Not specifying
    //   `starknet` dependency will error in 99% real-world Starknet contract projects.
    //   I think we can get away with emitting false positives for users who write raw contracts
    //   without using Starknet code generators. Such people shouldn't do what they do 😁
    if unit.main_component().target_kind() == TargetKind::STARKNET_CONTRACT
        && !has_plugin(db, is_starknet_plugin, unit.main_component())
    {
        ws.config().ui().warn(formatdoc! {
            r#"
            package `{package_name}` declares `starknet-contract` target, but does not depend on `starknet` package
            note: this may cause contract compilation to fail with cryptic errors
            help: add dependency on `starknet` to package manifest
             --> {scarb_toml}
                [dependencies]
                starknet = ">={cairo_version}"
            "#,
            scarb_toml=unit.main_component().package.manifest_path().workspace_relative(ws),
            cairo_version = crate::version::get().cairo.version,
        })
    }
}

fn suppress_error(err: &Error) -> bool {
    matches!(err.downcast_ref(), Some(&DiagnosticsError))
}

// Returns proc macro packages that need to be compiled for the provided Cairo compilation units.
pub fn plugins_required_for_units(units: &[CompilationUnit]) -> HashSet<PackageId> {
    units
        .iter()
        .flat_map(|unit| match unit {
            CompilationUnit::Cairo(unit) => unit
                .cairo_plugins
                .iter()
                .map(|p| p.package.id)
                .collect_vec(),
            _ => Vec::new(),
        })
        .collect::<HashSet<PackageId>>()
}

/// Returns true if the compilation unit is a proc-macro that is not required by any Cairo unit
/// and was not explicitly requested by the user via `--package`.
fn is_unused_proc_macro(
    unit: &CompilationUnit,
    required_plugins: &HashSet<PackageId>,
    required_packages: &[PackageId],
) -> bool {
    matches!(unit, CompilationUnit::ProcMacro(_))
        && !required_plugins.contains(&unit.main_package_id())
        && !required_packages.contains(&unit.main_package_id())
}

#[tracing::instrument(level = "trace", skip_all)]
fn collect_assets(unit: &CairoCompilationUnit) -> Result<Vec<(String, Utf8PathBuf)>> {
    // Map from file name -> (package name, absolute source path)
    let mut by_name: HashMap<String, (PackageId, Utf8PathBuf)> = HashMap::new();

    for component in &unit.components {
        let pkg = &component.package;
        let pkg_id = pkg.id;
        let mut seen_in_pkg: HashSet<String> = HashSet::new();
        for asset in pkg.assets()? {
            ensure!(
                asset.is_file(),
                "package `{pkg_id}` asset is not a file: {asset}"
            );
            let Some(file_name) = asset.file_name() else {
                bail!("package `{pkg_id}` asset path has no file name: {asset}");
            };
            ensure!(
                seen_in_pkg.insert(file_name.into()),
                "package `{pkg_id}` declares multiple assets with the same file name: {file_name}"
            );
            if let Some((other_pkg_id, _)) = by_name.get(file_name)
                && other_pkg_id != &pkg_id
            {
                bail!(
                    "multiple packages declare an asset with the same file name `{file_name}`: \
                    {other_pkg_id}, {pkg_id}"
                );
            }
            by_name.insert(file_name.into(), (pkg_id, asset));
        }
    }

    Ok(by_name
        .into_iter()
        .map(|(name, (_, path))| (name, path))
        .collect())
}

#[tracing::instrument(level = "trace", skip_all)]
fn copy_assets(assets: Vec<(String, Utf8PathBuf)>, target_dir: &Filesystem) -> Result<()> {
    let target_path = target_dir.path_existent()?;
    for (name, src) in assets {
        let dst = target_path.join(&name);
        trace!(%name, %src, %dst, "copying asset");
        fsx::copy(src, dst)?;
    }
    Ok(())
}
