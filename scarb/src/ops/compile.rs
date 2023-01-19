use std::time::Instant;
use std::{fs, mem};

use anyhow::{anyhow, Result};
use cairo_lang_compiler::project::{ProjectConfig, ProjectConfigContent};
use cairo_lang_compiler::{CompilerConfig, SierraProgram};
use indicatif::HumanDuration;

use crate::core::workspace::Workspace;
use crate::core::{Config, Package, PackageId};
use crate::ops;
use crate::ops::WorkspaceResolve;
use crate::ui::{Status, TypedMessage};

#[tracing::instrument(skip_all, level = "debug")]
pub fn compile(ws: &Workspace<'_>) -> Result<()> {
    let start_time = Instant::now();

    let resolve = ops::resolve_workspace(ws)?;

    // FIXME(mkaput): Iterate over all members here if current package is not set.
    let current_package = ws.current_package()?;

    ws.config().ui().print(Status::new(
        "Compiling",
        "green",
        &current_package.id.to_string(),
    ));

    compile_package(current_package, ws, &resolve).map_err(|err| {
        // TODO(mkaput): Make this an enum upstream.
        if err.to_string() == "Compilation failed." {
            anyhow!(
                "could not compile `{}` due to previous error",
                current_package.id.name
            )
        } else {
            err
        }
    })?;

    let elapsed_time = HumanDuration(start_time.elapsed());
    ws.config().ui().print(Status::new(
        "Finished",
        "green",
        &format!("release target(s) in {elapsed_time}"),
    ));

    Ok(())
}

fn compile_package(
    current_package: &Package,
    ws: &Workspace<'_>,
    resolve: &WorkspaceResolve,
) -> Result<()> {
    let project_config = build_project_config(current_package.id, resolve)?;

    let compiler_config = CompilerConfig {
        on_diagnostic: {
            // UNSAFE: We are not actually creating a dangling `Config` reference here,
            //   because diagnostic callback by definition should rather be dropped
            //   when compilation ends.
            let config: &'static Config = unsafe { mem::transmute(ws.config()) };
            Some(Box::new({
                |diagnostic: String| {
                    config
                        .ui()
                        .print(TypedMessage::naked_text("diagnostic", &diagnostic));
                }
            }))
        },
        ..CompilerConfig::default()
    };
    let sierra_program = run_compile(project_config, compiler_config)?;

    fs::write(
        ws.target_dir()
            .child("release")
            .as_existent()?
            .join(format!("{}.sierra", current_package.id.name)),
        sierra_program.to_string(),
    )?;

    Ok(())
}

fn build_project_config(member_id: PackageId, resolve: &WorkspaceResolve) -> Result<ProjectConfig> {
    let crate_roots = resolve
        .resolve
        .collect_compilation_unit_of(member_id)
        .iter()
        .map(|id| {
            let pkg = &resolve.packages[id];
            (pkg.id.name.clone(), pkg.source_dir().into_std_path_buf())
        })
        .collect();

    let content = ProjectConfigContent { crate_roots };

    Ok(ProjectConfig {
        base_path: resolve.packages[&member_id].root().into(),
        content,
    })
}

#[tracing::instrument(level = "trace", skip(compiler_config))]
fn run_compile(
    project_config: ProjectConfig,
    compiler_config: CompilerConfig,
) -> Result<SierraProgram> {
    cairo_lang_compiler::compile(project_config, compiler_config)
}
