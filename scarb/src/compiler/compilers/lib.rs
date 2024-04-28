use anyhow::{Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_casm::hints::Hint;
use cairo_lang_sierra::program::VersionedProgram;
use cairo_lang_sierra_to_casm::compiler::SierraToCasmConfig;
use cairo_lang_sierra_to_casm::metadata::{calc_metadata, calc_metadata_ap_change_only};
use cairo_lang_utils::bigint::BigIntAsHex;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace_span};

use crate::compiler::helpers::{
    build_compiler_config, collect_main_crate_ids, write_json, write_string,
};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, Compiler};
use crate::core::{TargetKind, Workspace};

pub struct LibCompiler;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Props {
    pub sierra: bool,
    pub casm: bool,
    pub sierra_text: bool,
    pub casm_text: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            sierra: true,
            casm: true,
            sierra_text: false,
            casm_text: false,
        }
    }
}

impl Compiler for LibCompiler {
    fn target_kind(&self) -> TargetKind {
        TargetKind::LIB.clone()
    }

    fn compile(
        &self,
        unit: CairoCompilationUnit,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let props: Props = unit.target().props()?;
        if !props.sierra && !props.casm && !props.sierra_text && !props.casm_text {
            ws.config().ui().warn(
                "Sierra, textual Sierra, CASM and textual CASM lib targets have been disabled, \
                Scarb will not produce anything",
            );
        }

        let target_dir = unit.target_dir(ws);

        let compiler_config = build_compiler_config(&unit, ws);

        let main_crate_ids = collect_main_crate_ids(&unit, db);

        let sierra_program: VersionedProgram = {
            let _ = trace_span!("compile_sierra").enter();
            let program_artifact = cairo_lang_compiler::compile_prepared_db_program_artifact(
                db,
                main_crate_ids,
                compiler_config,
            )?;
            program_artifact.into()
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
        let program = sierra_program.into_v1().unwrap().program;
        let metadata = {
            let _ = trace_span!("casm_calc_metadata").enter();

            if unit.compiler_config.enable_gas {
                debug!("calculating Sierra variables");
                calc_metadata(&program, Default::default())
            } else {
                debug!("calculating Sierra variables with no gas validation");
                calc_metadata_ap_change_only(&program)
            }
            .context("failed calculating Sierra variables")?
        };

        let cairo_program = {
            let _ = trace_span!("compile_casm").enter();
            let sierra_to_casm = SierraToCasmConfig {
                gas_usage_check: unit.compiler_config.enable_gas,
                max_bytecode_size: usize::MAX,
            };
            cairo_lang_sierra_to_casm::compiler::compile(&program, &metadata, sierra_to_casm)?
        };

        if props.casm {
            let assembled_cairo_program = cairo_program.assemble();
            let bytecode = assembled_cairo_program.bytecode.iter().map(|x| BigIntAsHex{ value: x.to_owned() }).collect::<Vec<BigIntAsHex>>();
            let hints = assembled_cairo_program.hints;
            let casm_program = SerializedCasm {
                bytecode,
                hints,
            };
            write_json(
                format!("{}.casm.json", unit.target().name).as_str(),
                "output file",
                &target_dir,
                ws,
                &casm_program,
            )
            .with_context(|| {
                format!("failed to serialize CASM program {}", unit.target().name)
            })?;
        }

        if props.casm_text {
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

/// Represents a contract in the Starknet network.
#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializedCasm {
    // #[serde(serialize_with = "serialize_big_uint", deserialize_with = "deserialize_big_uint")]
    // pub prime: BigUint,
    // pub compiler_version: String,
    pub bytecode: Vec<BigIntAsHex>,
    // #[serde(skip_serializing_if = "skip_if_none")]
    // pub bytecode_segment_lengths: Option<NestedIntList>,
    pub hints: Vec<(usize, Vec<Hint>)>,
    // pub entry_points_by_type: CasmContractEntryPoints,
}