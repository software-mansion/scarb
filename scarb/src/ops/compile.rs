use anyhow::{anyhow, Result};
use cairo_lang_compiler::diagnostics::DiagnosticsError;
use indoc::formatdoc;

use scarb_ui::components::Status;
use scarb_ui::HumanDuration;

use crate::compiler::db::{build_scarb_root_database, has_starknet_plugin};
use crate::compiler::CompilationUnit;
use crate::core::{PackageId, Utf8PathWorkspaceExt, Workspace};
use crate::ops;

#[tracing::instrument(skip_all, level = "debug")]
pub fn compile(packages: Vec<PackageId>, ws: &Workspace<'_>) -> Result<()> {
    let resolve = ops::resolve_workspace(ws)?;
    let compilation_units = ops::generate_compilation_units(&resolve, ws)?
        .into_iter()
        .filter(|cu| packages.contains(&cu.main_package_id))
        .collect::<Vec<_>>();

    for unit in compilation_units {
        compile_unit(unit, ws)?;
    }

    let elapsed_time = HumanDuration(ws.config().elapsed_time());
    ws.config().ui().print(Status::new(
        "Finished",
        &format!("release target(s) in {elapsed_time}"),
    ));

    Ok(())
}

fn compile_unit(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let package_name = unit.main_package_id.name.clone();

    ws.config()
        .ui()
        .print(Status::new("Compiling", &unit.name()));

    let mut db = build_scarb_root_database(&unit, ws)?;

    // NOTE: This is a special case that can be hit frequently by newcomers. Not specifying
    //   `starknet` dependency will error in 99% real-world Starknet contract projects.
    //   I think we can get away with emitting false positives for users who write raw contracts
    //   without using Starknet code generators. Such people shouldn't do what they do 😁
    if unit.target().kind == "starknet-contract" && !has_starknet_plugin(&db) {
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

fn suppress_error(err: &anyhow::Error) -> bool {
    matches!(err.downcast_ref(), Some(&DiagnosticsError))
}
