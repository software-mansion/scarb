use std::io::Write;
use std::mem;

use anyhow::Result;
use cairo_lang_compiler::project::{ProjectConfig, ProjectConfigContent};
use cairo_lang_compiler::{CompilerConfig, SierraProgram};

use crate::compiler::CompilationUnit;
use crate::core::{Config, Workspace};
use crate::ui::TypedMessage;

#[tracing::instrument(level = "trace", skip_all, fields(unit = unit.name()))]
pub fn compile_lib(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
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

    let target = unit.profile.target_dir(ws.config());
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
