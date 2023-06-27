use std::thread;

use anyhow::{anyhow, Result};
use cairo_lang_compiler::diagnostics::DiagnosticsError;
use indicatif::HumanDuration;
use indoc::formatdoc;

use crate::compiler::db::{build_scarb_root_database, has_starknet_plugin};
use crate::compiler::CompilationUnit;
use crate::core::{Utf8PathWorkspaceExt, Workspace};
use crate::ops;
use crate::ui::Status;

#[tracing::instrument(skip_all, level = "debug")]
pub fn compile(ws: &Workspace<'_>) -> Result<()> {
    let resolve = ops::resolve_workspace(ws)?;
    let compilation_units = ops::generate_compilation_units(&resolve, ws)?;

    for unit in compilation_units {
        compile_unit_isolated(unit, ws)?;
    }

    let elapsed_time = HumanDuration(ws.config().elapsed_time());
    ws.config().ui().print(Status::new(
        "Finished",
        &format!("release target(s) in {elapsed_time}"),
    ));

    Ok(())
}

// FIXME(mkaput): Remove this when Cairo will fix their issue upstream.
// NOTE: This is untested! Compiling such large Cairo files takes horribly long time.
/// Run compiler in a new thread which has significantly increased stack size.
/// The Cairo compiler tends to consume too much stack space in some specific cases:
/// https://github.com/starkware-libs/cairo/issues/3530.
/// It does not seem to consume infinite amounts though, so we try to confine it in arbitrarily
/// chosen big memory chunk.
fn compile_unit_isolated(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    thread::scope(|s| {
        thread::Builder::new()
            .name(format!("scarb compile {}", unit.id()))
            .stack_size(128 * 1024 * 1024)
            .spawn_scoped(s, || compile_unit(unit, ws))
            .expect("Failed to spawn compiler thread.")
            .join()
            .expect("Compiler thread has panicked.")
    })
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
