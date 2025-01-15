use crate::compiler::helpers::write_json;
use crate::compiler::helpers::{build_compiler_config, collect_main_crate_ids};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, Compiler};
use crate::core::{TargetKind, Workspace};
use anyhow::Result;
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_executable::executable::Executable;
use tracing::trace_span;

pub struct ExecutableCompiler;

impl Compiler for ExecutableCompiler {
    fn target_kind(&self) -> TargetKind {
        TargetKind::EXECUTABLE.clone()
    }

    fn compile(
        &self,
        unit: CairoCompilationUnit,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let target_dir = unit.target_dir(ws);
        let main_crate_ids = collect_main_crate_ids(&unit, db);
        let compiler_config = build_compiler_config(db, &unit, &main_crate_ids, ws);
        let span = trace_span!("compile_executable");
        let executable = {
            let _guard = span.enter();
            Executable::new(
                cairo_lang_executable::compile::compile_executable_in_prepared_db(
                    db,
                    None,
                    main_crate_ids,
                    compiler_config.diagnostics_reporter,
                )?,
            )
        };

        write_json(
            format!("{}.executable.json", unit.main_component().target_name()).as_str(),
            "output file",
            &target_dir,
            ws,
            &executable,
        )
    }
}
