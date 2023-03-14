//! Various utility functions helpful for interacting with Cairo compiler.

use anyhow::Result;
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_compiler::project::{ProjectConfig, ProjectConfigContent};
use cairo_lang_compiler::CompilerConfig;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::{CrateId, CrateLongId, Directory};
use tracing::trace;

use crate::compiler::CompilationUnit;
use crate::core::{PackageName, Workspace};
use crate::ui::TypedMessage;

pub fn build_project_config(unit: &CompilationUnit) -> Result<ProjectConfig> {
    let crate_roots = unit
        .components
        .iter()
        .filter(|pkg| pkg.id.name != PackageName::CORE)
        .map(|pkg| {
            (
                pkg.id.name.to_smol_str(),
                pkg.source_dir().into_std_path_buf(),
            )
        })
        .collect();

    let corelib = unit
        .components
        .iter()
        .find(|pkg| pkg.id.name == PackageName::CORE)
        .map(|pkg| Directory(pkg.source_dir().into_std_path_buf()));

    let content = ProjectConfigContent { crate_roots };

    let project_config = ProjectConfig {
        base_path: unit.package.root().into(),
        corelib,
        content,
    };

    trace!(?project_config);

    Ok(project_config)
}

pub fn build_compiler_config<'c>(unit: &CompilationUnit, ws: &Workspace<'c>) -> CompilerConfig<'c> {
    CompilerConfig {
        diagnostics_reporter: DiagnosticsReporter::callback({
            let config = ws.config();
            |diagnostic: String| {
                config
                    .ui()
                    .print(TypedMessage::naked_text("diagnostic", &diagnostic));
            }
        }),
        replace_ids: unit.compiler_config.sierra_replace_ids,
        ..CompilerConfig::default()
    }
}

pub fn collect_main_crate_ids(unit: &CompilationUnit, db: &RootDatabase) -> Vec<CrateId> {
    vec![db.intern_crate(CrateLongId(unit.package.id.name.to_smol_str()))]
}
