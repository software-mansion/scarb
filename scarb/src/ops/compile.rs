use anyhow::{anyhow, Context, Error, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::DiagnosticsError;
use cairo_lang_utils::Upcast;
use indoc::formatdoc;
use itertools::Itertools;
use scarb_ui::args::FeaturesSpec;
use scarb_ui::components::Status;
use scarb_ui::HumanDuration;
use smol_str::{SmolStr, ToSmolStr};
use std::thread;

use crate::compiler::db::{build_scarb_root_database, has_starknet_plugin, ScarbDatabase};
use crate::compiler::helpers::{build_compiler_config, collect_main_crate_ids};
use crate::compiler::plugin::proc_macro;
use crate::compiler::{CairoCompilationUnit, CompilationUnit, CompilationUnitAttributes};
use crate::core::{
    FeatureName, PackageId, PackageName, TargetKind, Utf8PathWorkspaceExt, Workspace,
};
use crate::ops;
use crate::ops::{get_test_package_ids, validate_features};

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
    process(packages, opts, ws, compile_unit, None)
}

#[tracing::instrument(skip_all, level = "debug")]
pub fn check(packages: Vec<PackageId>, opts: CompileOpts, ws: &Workspace<'_>) -> Result<()> {
    process(packages, opts, ws, check_unit, Some("checking"))
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
    F: FnMut(CompilationUnit, &Workspace<'_>) -> Result<()>,
{
    let resolve = ops::resolve_workspace(ws)?;
    let packages_to_process = ws
        .members()
        .filter(|p| packages.contains(&p.id))
        .collect_vec();
    validate_features(&packages_to_process, &opts.features)?;
    // Add test compilation units to build
    let packages = get_test_package_ids(packages, ws);
    let compilation_units =
        ops::generate_compilation_units(&resolve, &opts.features, opts.ignore_cairo_version, ws)?
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
                            .iter()
                            .any(|t| opts.include_target_names.contains(&t.name)));
                let is_selected = packages.contains(&cu.main_package_id());
                let is_cairo_plugin = matches!(cu, CompilationUnit::ProcMacro(_));
                is_cairo_plugin || (is_selected && is_included && !is_excluded)
            })
            .sorted_by_key(|cu| {
                if matches!(cu, CompilationUnit::ProcMacro(_)) {
                    0
                } else {
                    1
                }
            })
            .collect::<Vec<_>>();

    for unit in compilation_units {
        operation(unit, ws)?;
    }

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

fn compile_unit_inner(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let package_name = unit.main_package_id().name.clone();

    ws.config()
        .ui()
        .print(Status::new("Compiling", &unit.name()));

    let result = match unit {
        CompilationUnit::ProcMacro(unit) => proc_macro::compile_unit(unit, ws),
        CompilationUnit::Cairo(unit) => {
            let ScarbDatabase {
                mut db,
                proc_macro_host,
            } = build_scarb_root_database(&unit, ws)?;
            check_starknet_dependency(&unit, ws, &db, &package_name);
            let result = ws.config().compilers().compile(unit, &mut db, ws);
            proc_macro_host
                .post_process(db.upcast())
                .context("procedural macro post processing callback failed")?;
            result
        }
    };

    result.map_err(|err| {
        if !suppress_error(&err) {
            ws.config().ui().anyhow(&err);
        }

        anyhow!("could not compile `{package_name}` due to previous error")
    })
}

fn check_unit(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let package_name = unit.main_package_id().name.clone();

    ws.config()
        .ui()
        .print(Status::new("Checking", &unit.name()));

    let result = match unit {
        CompilationUnit::ProcMacro(unit) => proc_macro::check_unit(unit, ws),
        CompilationUnit::Cairo(unit) => {
            let ScarbDatabase { db, .. } = build_scarb_root_database(&unit, ws)?;
            let main_crate_ids = collect_main_crate_ids(&unit, &db);
            check_starknet_dependency(&unit, ws, &db, &package_name);
            let mut compiler_config = build_compiler_config(&db, &unit, &main_crate_ids, ws);
            compiler_config
                .diagnostics_reporter
                .ensure(&db)
                .map_err(|err| err.into())
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
    db: &RootDatabase,
    package_name: &PackageName,
) {
    // NOTE: This is a special case that can be hit frequently by newcomers. Not specifying
    //   `starknet` dependency will error in 99% real-world Starknet contract projects.
    //   I think we can get away with emitting false positives for users who write raw contracts
    //   without using Starknet code generators. Such people shouldn't do what they do 😁
    if unit.main_component().target_kind() == TargetKind::STARKNET_CONTRACT
        && !has_starknet_plugin(db)
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

fn suppress_error(err: &anyhow::Error) -> bool {
    matches!(err.downcast_ref(), Some(&DiagnosticsError))
}
