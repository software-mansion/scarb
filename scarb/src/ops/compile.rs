use std::io::Write;
use std::mem;

use anyhow::{anyhow, Result};
use cairo_lang_compiler::project::{ProjectConfig, ProjectConfigContent};
use cairo_lang_compiler::{CompilerConfig, SierraProgram};
use indicatif::HumanDuration;

use crate::compiler::CompilationUnit;
use crate::core::workspace::Workspace;
use crate::core::Config;
use crate::ops;
use crate::ui::{Status, TypedMessage};

#[tracing::instrument(skip_all, level = "debug")]
pub fn compile(ws: &Workspace<'_>) -> Result<()> {
    let resolve = ops::resolve_workspace(ws)?;
    let compilation_units = ops::generate_compilation_units(&resolve, ws)?;

    // TODO(mkaput): Parallelize this loop.
    //   Caveat: This shouldn't be just rayon::map call, because we will introduce dependencies
    //   between compilation units in the future.
    for unit in compilation_units {
        compile_unit(unit, ws)?;
    }

    let elapsed_time = HumanDuration(ws.config().elapsed_time());
    ws.config().ui().print(Status::new(
        "Finished",
        "green",
        &format!("release target(s) in {elapsed_time}"),
    ));

    Ok(())
}

fn compile_unit(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let package_name = unit.package.id.name.clone();

    ws.config()
        .ui()
        .print(Status::new("Compiling", "green", &unit.name()));

    compile_unit_impl(unit, ws).map_err(|err| {
        // TODO(mkaput): Make this an enum upstream.
        if err.to_string() == "Compilation failed." {
            anyhow!("could not compile `{package_name}` due to previous error")
        } else {
            err
        }
    })?;

    Ok(())
}

// TODO(mkaput): Compile each kind appropriately.
fn compile_unit_impl(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let project_config = build_project_config(&unit)?;

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

    let target = ws.target_dir().child("release");
    let mut file = target.open_rw(
        format!("{}.sierra", unit.target.name),
        "output file",
        ws.config(),
    )?;
    file.write_all(sierra_program.to_string().as_bytes())?;

    Ok(())
}

fn build_project_config(unit: &CompilationUnit) -> Result<ProjectConfig> {
    let crate_roots = unit
        .components
        .iter()
        .map(|pkg| {
            (
                pkg.id.name.to_smol_str(),
                pkg.source_dir().into_std_path_buf(),
            )
        })
        .collect();

    let content = ProjectConfigContent { crate_roots };

    Ok(ProjectConfig {
        base_path: unit.package.root().into(),
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
