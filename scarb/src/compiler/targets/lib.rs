use std::io::Write;
use std::mem;

use anyhow::{Context, Result};
use cairo_lang_compiler::project::{ProjectConfig, ProjectConfigContent};
use cairo_lang_compiler::{CompilerConfig, SierraProgram};
use cairo_lang_sierra_to_casm::compiler::CairoProgram;
use cairo_lang_sierra_to_casm::metadata::calc_metadata;

use crate::compiler::CompilationUnit;
use crate::core::{Config, LibTargetKind, Workspace};
use crate::ui::TypedMessage;

#[tracing::instrument(level = "trace", skip_all, fields(unit = unit.name()))]
pub fn compile_lib(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let props = unit.target.kind.downcast::<LibTargetKind>();
    if !props.sierra && !props.casm {
        ws.config().ui().warn(
            "both Sierra and CASM lib targets have been disabled, \
            Scarb will not produce anything",
        );
    }

    let target_dir = unit.profile.target_dir(ws.config());

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

    let sierra_program = compile_sierra(project_config, compiler_config)?;

    if props.sierra {
        let mut file = target_dir.open_rw(
            format!("{}.sierra", unit.target.name),
            "output file",
            ws.config(),
        )?;
        file.write_all(sierra_program.to_string().as_bytes())?;
    }

    if props.casm {
        let cairo_program = compile_casm(&sierra_program)?;

        let mut file = target_dir.open_rw(
            format!("{}.casm", unit.target.name),
            "output file",
            ws.config(),
        )?;
        file.write_all(cairo_program.to_string().as_bytes())?;
    }

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
fn compile_sierra(
    project_config: ProjectConfig,
    compiler_config: CompilerConfig,
) -> Result<SierraProgram> {
    cairo_lang_compiler::compile(project_config, compiler_config)
}

#[tracing::instrument(level = "trace", skip_all)]
fn compile_casm(sierra_program: &SierraProgram) -> Result<CairoProgram> {
    use cairo_lang_sierra_to_casm::compiler::compile;

    let gas_usage_check = true;
    let metadata = calc_metadata(sierra_program).context("failed calculating Sierra variables")?;
    let cairo_program = compile(sierra_program, &metadata, gas_usage_check)?;
    Ok(cairo_program)
}
