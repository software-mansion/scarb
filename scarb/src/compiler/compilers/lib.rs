use std::io::Write;

use anyhow::{Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_sierra_to_casm::metadata::{calc_metadata, MetadataComputationConfig};
use tracing::trace_span;

use crate::compiler::helpers::{
    build_compiler_config, build_project_config, collect_main_crate_ids,
};
use crate::compiler::{CompilationUnit, Compiler};
use crate::core::{LibTargetKind, Workspace};

pub struct LibCompiler;

impl Compiler for LibCompiler {
    fn target_kind(&self) -> &str {
        "lib"
    }

    fn compile(&self, unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
        let props = unit.target.kind.downcast::<LibTargetKind>();
        if !props.sierra && !props.casm {
            ws.config().ui().warn(
                "both Sierra and CASM lib targets have been disabled, \
                Scarb will not produce anything",
            );
        }

        let target_dir = unit.profile.target_dir(ws.config());

        let mut db = RootDatabase::builder()
            .with_project_config(build_project_config(&unit)?)
            .build()?;

        let compiler_config = build_compiler_config(&unit, ws);

        let main_crate_ids = collect_main_crate_ids(&unit, &db);

        let sierra_program = {
            let _ = trace_span!("compile_sierra").enter();
            cairo_lang_compiler::compile_prepared_db(&mut db, main_crate_ids, compiler_config)?
        };

        if props.sierra {
            let mut file = target_dir.open_rw(
                format!("{}.sierra", unit.target.name),
                "output file",
                ws.config(),
            )?;
            file.write_all(sierra_program.to_string().as_bytes())?;
        }

        if props.casm {
            let gas_usage_check = true;

            let metadata = {
                let _ = trace_span!("casm_calc_metadata");
                calc_metadata(&sierra_program, MetadataComputationConfig::default())
                    .context("failed calculating Sierra variables")?
            };

            let cairo_program = {
                let _ = trace_span!("compile_casm");
                cairo_lang_sierra_to_casm::compiler::compile(
                    &sierra_program,
                    &metadata,
                    gas_usage_check,
                )?
            };

            let mut file = target_dir.open_rw(
                format!("{}.casm", unit.target.name),
                "output file",
                ws.config(),
            )?;
            file.write_all(cairo_program.to_string().as_bytes())?;
        }

        Ok(())
    }
}
