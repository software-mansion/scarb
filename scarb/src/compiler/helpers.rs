//! Various utility functions helpful for interacting with Cairo compiler.

use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_compiler::CompilerConfig;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::{CrateId, CrateLongId};

use scarb_ui::components::TypedMessage;

use crate::compiler::CompilationUnit;
use crate::core::Workspace;

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
    vec![db.intern_crate(CrateLongId::Real(
        unit.main_component().cairo_package_name(),
    ))]
}
