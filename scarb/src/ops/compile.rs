use anyhow::{anyhow, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::DiagnosticsError;
use indoc::formatdoc;
use itertools::Itertools;

use scarb_ui::components::Status;
use scarb_ui::HumanDuration;

use crate::compiler::db::{build_scarb_root_database, has_starknet_plugin};
use crate::compiler::helpers::build_compiler_config;
use crate::compiler::CompilationUnit;
use crate::core::{PackageId, PackageName, TargetKind, Utf8PathWorkspaceExt, Workspace};
use crate::ops;

#[derive(Debug)]
pub struct CompileOpts {
    pub include_targets: Vec<TargetKind>,
    pub exclude_targets: Vec<TargetKind>,
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

    // Add test compilation units to build
    let packages = packages
        .into_iter()
        .flat_map(|package_id| {
            let package = ws.members().find(|p| p.id == package_id).unwrap();
            let mut result: Vec<PackageId> = package
                .manifest
                .targets
                .iter()
                .map(|t| package.id.for_test_target(t.name.clone()))
                .collect();
            result.push(package_id);
            result
        })
        .collect::<Vec<PackageId>>();

    let compilation_units = ops::generate_compilation_units(&resolve, ws)?
        .into_iter()
        .filter(|cu| {
            let is_excluded = opts.exclude_targets.contains(&cu.target().kind);
            let is_included =
                opts.include_targets.is_empty() || opts.include_targets.contains(&cu.target().kind);
            let is_selected = packages.contains(&cu.main_package_id);
            let is_cairo_plugin = cu.components.first().unwrap().target.is_cairo_plugin();
            is_cairo_plugin || (is_selected && is_included && !is_excluded)
        })
        .sorted_by_key(|cu| {
            if cu.components.first().unwrap().target.is_cairo_plugin() {
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
    let formatted_message = match operation_type {
        Some(op) => format!("{op} release target(s) in {elapsed_time}"),
        None => format!("release target(s) in {elapsed_time}"),
    };
    ws.config()
        .ui()
        .print(Status::new("Finished", &formatted_message));

    Ok(())
}

fn compile_unit(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let package_name = unit.main_package_id.name.clone();

    ws.config()
        .ui()
        .print(Status::new("Compiling", &unit.name()));

    let mut db = build_scarb_root_database(&unit, ws)?;

    check_starknet_dependency(&unit, ws, &db, &package_name);

    ws.config()
        .compilers()
        .compile(unit, &mut db, ws)
        .map_err(|err| {
            if !suppress_error(&err) {
                ws.config().ui().anyhow(&err);
            }

            anyhow!("could not compile `{package_name}` due to previous error")
        })?;

    Ok(())
}

fn check_unit(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let package_name = unit.main_package_id.name.clone();

    ws.config()
        .ui()
        .print(Status::new("Checking", &unit.name()));

    let db = build_scarb_root_database(&unit, ws)?;

    check_starknet_dependency(&unit, ws, &db, &package_name);

    let mut compiler_config = build_compiler_config(&unit, ws);

    compiler_config
        .diagnostics_reporter
        .ensure(&db)
        .map_err(|err| {
            let valid_error = err.into();
            if !suppress_error(&valid_error) {
                ws.config().ui().anyhow(&valid_error);
            }

            anyhow!("could not check `{package_name}` due to previous error")
        })?;

    Ok(())
}

fn check_starknet_dependency(
    unit: &CompilationUnit,
    ws: &Workspace<'_>,
    db: &RootDatabase,
    package_name: &PackageName,
) {
    // NOTE: This is a special case that can be hit frequently by newcomers. Not specifying
    //   `starknet` dependency will error in 99% real-world Starknet contract projects.
    //   I think we can get away with emitting false positives for users who write raw contracts
    //   without using Starknet code generators. Such people shouldn't do what they do 😁
    if unit.target().kind == TargetKind::STARKNET_CONTRACT && !has_starknet_plugin(db) {
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
