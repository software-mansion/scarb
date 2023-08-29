use anyhow::Result;
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_test_plugin::compile_test_prepared_db;
use scarb_ui::components::TypedMessage;

use crate::compiler::helpers::{collect_all_crate_ids, collect_main_crate_ids, write_json};
use crate::compiler::{CompilationUnit, Compiler};
use crate::core::{PackageName, SourceId, Target, Workspace};

pub struct TestCompiler;

impl Compiler for TestCompiler {
    fn target_kind(&self) -> &str {
        Target::TEST
    }

    fn compile(
        &self,
        unit: CompilationUnit,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let target_dir = unit.target_dir(ws);

        let test_crate_ids = collect_main_crate_ids(&unit, db);
        let main_crate_ids = collect_all_crate_ids(&unit, db);
        let starknet = unit.cairo_plugins.iter().any(|plugin| {
            plugin.package.id.name == PackageName::STARKNET
                && plugin.package.id.source_id == SourceId::for_std()
        });

        let diagnostics_reporter = DiagnosticsReporter::callback({
            let config = ws.config();
            |diagnostic: String| {
                config
                    .ui()
                    .print(TypedMessage::naked_text("diagnostic", &diagnostic));
            }
        });

        diagnostics_reporter
            .with_extra_crates(&main_crate_ids)
            .ensure(db)?;

        let test_compilation =
            compile_test_prepared_db(db, starknet, main_crate_ids, test_crate_ids)?;

        let file_name = format!("{}.test.json", unit.target().name);
        write_json(
            &file_name,
            "output file",
            &target_dir,
            ws,
            &test_compilation,
        )?;

        Ok(())
    }
}
