use crate::compiler::helpers::write_json;
use crate::compiler::helpers::{build_compiler_config, collect_main_crate_ids};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, Compiler};
use crate::core::{TargetKind, Utf8PathWorkspaceExt, Workspace};
use anyhow::{ensure, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_executable::executable::Executable;
use indoc::formatdoc;
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
        ensure!(
            !unit.compiler_config.enable_gas,
            formatdoc! {r#"
                executable target cannot be compiled with enabled gas calculation
                help: if you want to diable gas calculation, consider adding following
                excerpt to your package manifest
                    -> {scarb_toml}
                        [cairo]
                        enable-gas = false
                "#, scarb_toml=unit.main_component().package.manifest_path().workspace_relative(ws)}
        );

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
