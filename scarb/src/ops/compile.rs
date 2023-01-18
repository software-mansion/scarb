use std::fs;

use anyhow::Result;
use cairo_lang_compiler::project::{ProjectConfig, ProjectConfigContent};
use cairo_lang_compiler::{CompilerConfig, SierraProgram};

use crate::core::workspace::Workspace;
use crate::core::PackageId;
use crate::ops;
use crate::ops::WorkspaceResolve;

#[tracing::instrument(skip_all, level = "debug")]
pub fn compile(ws: &Workspace<'_>, on_diagnostic: Box<dyn FnMut(String)>) -> Result<()> {
    // FIXME(mkaput): Iterate over all members here if current package is not set.
    let current_package = ws.current_package()?;
    let resolve = ops::resolve_workspace(ws)?;
    let project_config = build_project_config(current_package.id, &resolve)?;

    let sierra_program = run_compile(project_config, on_diagnostic)?;

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

#[tracing::instrument(level = "trace", skip(on_diagnostic))]
fn run_compile(
    project_config: ProjectConfig,
    on_diagnostic: Box<dyn FnMut(String)>,
) -> Result<SierraProgram> {
    cairo_lang_compiler::compile(
        project_config,
        CompilerConfig {
            on_diagnostic: Some(on_diagnostic),
            ..CompilerConfig::default()
        },
    )
}
