use anyhow::{Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_sierra_to_casm::compiler::SierraToCasmConfig;
use cairo_lang_sierra_to_casm::metadata::calc_metadata;
use serde::{Deserialize, Serialize};
use tracing::trace_span;

use crate::compiler::compilers::MAX_BYTECODE_SIZE;
use crate::compiler::helpers::{
    build_compiler_config, collect_main_crate_ids, write_json, write_string,
};
use crate::compiler::{CompilationUnit, Compiler};
use crate::core::{TargetKind, Workspace};

pub struct LibCompiler;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Props {
    pub sierra: bool,
    pub casm: bool,
    pub sierra_text: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            sierra: true,
            casm: false,
            sierra_text: false,
        }
    }
}

impl Compiler for LibCompiler {
    fn target_kind(&self) -> TargetKind {
        TargetKind::LIB.clone()
    }

    fn compile(
        &self,
        unit: CompilationUnit,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let props: Props = unit.target().props()?;
        if !props.sierra && !props.casm && !props.sierra_text {
            ws.config().ui().warn(
                "Sierra, textual Sierra and CASM lib targets have been disabled, \
                Scarb will not produce anything",
            );
        }

        let target_dir = unit.target_dir(ws);

        let compiler_config = build_compiler_config(&unit, ws);

        let main_crate_ids = collect_main_crate_ids(&unit, db);

        let sierra_program = {
            let _ = trace_span!("compile_sierra").enter();
            cairo_lang_compiler::compile_prepared_db(db, main_crate_ids, compiler_config)?
                .into_artifact()
        };

        if props.sierra {
            write_json(
                format!("{}.sierra.json", unit.target().name).as_str(),
                "output file",
                &target_dir,
                ws,
                &sierra_program,
            )
            .with_context(|| {
                format!("failed to serialize Sierra program {}", unit.target().name)
            })?;
        }

        if props.sierra_text {
            write_string(
                format!("{}.sierra", unit.target().name).as_str(),
                "output file",
                &target_dir,
                ws,
                &sierra_program,
            )?;
        }

        if props.casm {
            let program = sierra_program.into_v1().unwrap().program;

            let gas_usage_check = true;

            let metadata = {
                let _ = trace_span!("casm_calc_metadata").enter();
                calc_metadata(&program, Default::default())
                    .context("failed calculating Sierra variables")?
            };

            let cairo_program = {
                let _ = trace_span!("compile_casm").enter();
                let config = SierraToCasmConfig {
                    gas_usage_check,
                    max_bytecode_size: MAX_BYTECODE_SIZE,
                };
                cairo_lang_sierra_to_casm::compiler::compile(&program, &metadata, config)?
            };

            write_string(
                format!("{}.casm", unit.target().name).as_str(),
                "output file",
                &target_dir,
                ws,
                cairo_program,
            )?;
        }

        Ok(())
    }
}
