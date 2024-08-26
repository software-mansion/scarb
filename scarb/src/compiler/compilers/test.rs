use anyhow::Result;
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_sierra::program::VersionedProgram;
use cairo_lang_test_plugin::{compile_test_prepared_db, TestsCompilationConfig};
use tracing::trace_span;

use crate::compiler::helpers::{
    build_compiler_config, collect_all_crate_ids, collect_main_crate_ids, write_json,
};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, Compiler};
use crate::core::{PackageName, SourceId, TargetKind, Workspace};

pub struct TestCompiler;

impl Compiler for TestCompiler {
    fn target_kind(&self) -> TargetKind {
        TargetKind::TEST.clone()
    }

    fn compile(
        &self,
        unit: CairoCompilationUnit,
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

        let diagnostics_reporter = build_compiler_config(&unit, ws).diagnostics_reporter;

        diagnostics_reporter
            .with_crates(&main_crate_ids)
            .ensure(db)?;

        let test_compilation = {
            let _ = trace_span!("compile_test").enter();
            let config = TestsCompilationConfig {
                starknet,
                add_statements_functions: unit
                    .compiler_config
                    .unstable_add_statements_functions_debug_info,
                add_statements_code_locations: false,
            };
            let allow_warnings = unit.compiler_config.allow_warnings;
            compile_test_prepared_db(db, config, main_crate_ids, test_crate_ids, allow_warnings)?
        };

        {
            let _ = trace_span!("serialize_test").enter();

            let sierra_program: VersionedProgram = test_compilation.sierra_program.clone().into();
            let file_name = format!("{}.test.sierra.json", unit.main_component().target_name());
            write_json(&file_name, "output file", &target_dir, ws, &sierra_program)?;

            let file_name = format!("{}.test.json", unit.main_component().target_name());
            write_json(
                &file_name,
                "output file",
                &target_dir,
                ws,
                &test_compilation.metadata,
            )?;
        }

        Ok(())
    }
}
