use std::fs;

use anyhow::Result;
use cairo_lang_compiler::project::{ProjectConfig, ProjectConfigContent};
use cairo_lang_compiler::CompilerConfig;

use crate::core::workspace::Workspace;
use crate::ops;

#[tracing::instrument(skip_all, level = "debug")]
pub fn compile(ws: &Workspace<'_>, on_diagnostic: Box<dyn FnMut(String)>) -> Result<()> {
    let project_config = build_project_config(ws)?;

    let sierra_program = cairo_lang_compiler::compile(
        project_config,
        CompilerConfig {
            on_diagnostic: Some(on_diagnostic),
            ..CompilerConfig::default()
        },
    )?;

    fs::write(
        ws.target_dir()
            .child("release")
            .as_existent()?
            .join(format!("{}.sierra", ws.current_package()?.id.name)),
        sierra_program.to_string(),
    )?;

    Ok(())
}

fn build_project_config(ws: &Workspace<'_>) -> Result<ProjectConfig> {
    let resolve = ops::resolve_workspace(ws)?;

    let crate_roots = resolve
        .packages
        .values()
        .map(|pkg| (pkg.id.name.clone(), pkg.source_dir()))
        .collect();

    let content = ProjectConfigContent { crate_roots };

    Ok(ProjectConfig {
        base_path: ws.root().into(),
        content,
    })
}
