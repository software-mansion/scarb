use crate::compiler::db::{
    ScarbDatabase, build_scarb_root_database, has_plugin, is_starknet_plugin,
};
use crate::compiler::helpers::{build_compiler_config, collect_main_crate_ids};
use crate::compiler::incremental::artifacts_fingerprint::{
    UnitArtifactsFingerprint, artifacts_fingerprint_allowed, load_unit_artifacts_local_paths,
    save_unit_artifacts_fingerprint, unit_artifacts_fingerprint_is_fresh,
};
use crate::compiler::incremental::{
    IncrementalContext, load_incremental_artifacts, save_incremental_artifacts,
};
use crate::compiler::plugin::proc_macro;
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
use cairo_lang_compiler::diagnostics::DiagnosticsError;
use camino::Utf8PathBuf;
use indoc::formatdoc;
use itertools::Itertools;
use salsa::Database;
use scarb_ui::HumanDuration;
use scarb_ui::args::FeaturesSpec;
use scarb_ui::components::Status;
use smol_str::{SmolStr, ToSmolStr};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
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

    for unit in units {
        // We can skip compiling proc macros that are not used by Cairo compilation units.
        if matches!(&unit, &CompilationUnit::ProcMacro(_))
            && !required_plugins.contains(&unit.main_package_id())
            // Unless they are explicitly requested with `--package` CLI arg.
            && !required_packages.contains(&unit.main_package_id())
        {
            continue;
        }
        compile_unit(unit, ws)?;
    }
    Ok(())
}

/// Run compiler in a new thread.
/// The stack size of created threads can be altered with `RUST_MIN_STACK` env variable.
pub fn compile_unit(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    thread::scope(|s| {
        thread::Builder::new()
            .name(format!("scarb compile {}", unit.id()))
            .spawn_scoped(s, || compile_unit_inner(unit, ws))
            .expect("Failed to spawn compiler thread.")
            .join()
            .expect("Compiler thread has panicked.")
    })
}

#[tracing::instrument(skip_all, level = "trace")]
fn compile_unit_inner(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
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
        CompilationUnit::Cairo(unit) => compile_cairo_unit_inner(unit, ws),
    };

    result.map_err(|err| {
        if !suppress_error(&err) {
            ws.config().ui().anyhow(&err);
        }
        anyhow!(
            "could not compile `{package_name}` due to {}",
            ws.config().ui().format_diagnostic_counts()
        )
    })
}

fn compile_cairo_unit_inner(unit: CairoCompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let package_name = unit.main_package_id().name.clone();
    ws.config()
        .ui()
        .print(Status::new("Compiling", &unit.name()));
    let ScarbDatabase {
        mut db,
        proc_macros,
    } = build_scarb_root_database(&unit, ws, Default::default())?;
    check_starknet_dependency(&unit, ws, &db, &package_name);
    let assets = collect_assets(&unit)?;

    // This scope limits the offloader lifetime.
    thread::scope(|s| {
        let offloader = Offloader::new(s, ws);
        let target_dir = unit.target_dir(ws);

        let ctx = load_incremental_artifacts(&unit, &mut db, ws)?;

        let warnings_to_print = ws.config().ui().verbosity().should_print_warnings()
            && !ctx.cached_crates_with_warnings().is_empty();

        let is_fresh_unit_artifacts = !warnings_to_print
            && artifacts_fingerprint_allowed()
            && ctx.cached_crates_with_warnings().is_empty()
            && ctx
                .fingerprints()
                .and_then(|unit_fingerprint| {
                    load_unit_artifacts_local_paths(&unit, ws)
                        .transpose()
                        .map(|artifacts| {
                            let fingerprint =
                                UnitArtifactsFingerprint::new(&unit, unit_fingerprint, artifacts?);
                            anyhow::Ok(unit_artifacts_fingerprint_is_fresh(&unit, fingerprint, ws)?)
                        })
                })
                .transpose()?
                .unwrap_or_default();

        let ctx = Arc::new(ctx);
        if !is_fresh_unit_artifacts {
            ws.config()
                .compilers()
                .compile(&unit, ctx.clone(), &offloader, &mut db, ws)?;
            save_incremental_artifacts(&unit, &db, ctx.clone(), ws)?;

            for plugin in proc_macros {
                plugin
                    .post_process(&db)
                    .context("procedural macro post processing callback failed")?;
            }
        };

        let span = trace_span!("drop_db");
        {
            let _guard = span.enter();
            drop(db);
        }

        let span = trace_span!("offloader_join");
        {
            let _guard = span.enter();
            offloader.join()?;
        }

        if artifacts_fingerprint_allowed()
            && !is_fresh_unit_artifacts
            && let Some(unit_fingerprint) = ctx.fingerprints()
        {
            let fingerprint =
                UnitArtifactsFingerprint::new(&unit, unit_fingerprint, ctx.artifacts());
            save_unit_artifacts_fingerprint(&unit, fingerprint, ws)?;
        }

        copy_assets(assets, &target_dir)?;

        Ok(())
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
            compile_unit(unit, ws)?;
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
        anyhow!(
            "could not check `{package_name}` due to {}",
            ws.config().ui().format_diagnostic_counts()
        )
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
            if let Some((other_pkg_id, other_asset)) = by_name.get(file_name)
                && other_pkg_id != &pkg_id
                // Allow multiple compilation unit components to declare the same asset name if they
                // point to exactly the same file. This makes it possible to compile package targets
                // where a single package is spread into many units (such as integration tests).
                // This condition is unsound, as one package may use some path shenanigans in its
                // Scarb.toml to refer to another package's asset, but this is deliberate, malicious
                // behavior that has no bad effects anyway, and we can accept the cost.
                && other_asset != &asset
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
