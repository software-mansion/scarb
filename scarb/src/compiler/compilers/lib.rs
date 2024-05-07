use anyhow::{Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::CompilerConfig;
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_sierra::program::VersionedProgram;
use cairo_lang_sierra_to_casm::compiler::SierraToCasmConfig;
use cairo_lang_sierra_to_casm::metadata::{calc_metadata, calc_metadata_ap_change_only};
use indoc::formatdoc;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace_span};

use crate::compiler::helpers::{
    build_compiler_config, collect_main_crate_ids, write_json, write_string,
};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, Compiler};
use crate::core::{TargetKind, Utf8PathWorkspaceExt, Workspace};

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
        unit: CairoCompilationUnit,
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

        validate_compiler_config(db, &compiler_config, &unit, ws);

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

        if props.casm {
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

fn validate_compiler_config(
    db: &RootDatabase,
    compiler_config: &CompilerConfig<'_>,
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
) {
    // Generally, lib target compilation should be driven by a certain objective (e.g. cairo-run,
    // test framework, etc.), expressed by the plugin set with executables definition.
    // This does not apply to debug build (expressed by `replace_ids` flag),
    // which is a goal by itself.
    // See starkware-libs/cairo#5440 for more context.
    let executable_plugin = db
        .macro_plugins()
        .iter()
        .any(|plugin| !plugin.executable_attributes().is_empty());
    if !executable_plugin && !compiler_config.replace_ids {
        ws.config().ui().warn(formatdoc! {r#"
            artefacts produced by this build may be hard to utilize due to the build configuration
            please make sure your build configuration is correct
            help: if you want to use your build with a specialized tool that runs Sierra code (for
            instance with a test framework like Forge), please make sure all required dependencies
            are specified in your package manifest.
            help: if you want to compile a Starknet contract, make sure to use the `starknet-contract`
            target, by adding following excerpt to your package manifest
            -> {scarb_toml}
                [[target.starknet-contract]]
            help: if you want to read the generated Sierra code yourself, consider enabling
            the debug names, by adding the following excerpt to your package manifest.
            -> {scarb_toml}
                [cairo]
                sierra-replace-ids = true"#, scarb_toml=unit.main_component().package.manifest_path().workspace_relative(ws),
        }, );
    }
}
