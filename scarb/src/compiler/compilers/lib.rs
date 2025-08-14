use anyhow::{Context, Result};
use cairo_lang_compiler::CompilerConfig;
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::plugin::MacroPlugin;
use cairo_lang_filesystem::ids::CrateId;
use cairo_lang_sierra::program::VersionedProgram;
use cairo_lang_sierra_to_casm::compiler::SierraToCasmConfig;
use cairo_lang_sierra_to_casm::metadata::{calc_metadata, calc_metadata_ap_change_only};
use indoc::formatdoc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, trace_span};

use crate::compiler::helpers::{
    build_compiler_config, collect_main_crate_ids, write_json, write_string,
};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, Compiler};
use crate::core::{TargetKind, Utf8PathWorkspaceExt, Workspace};
use crate::internal::offloader::Offloader;

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
        unit: &CairoCompilationUnit,
        cached_crates: &[CrateId],
        offloader: &Offloader<'_>,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let props: Props = unit.main_component().targets.target_props()?;
        if !props.sierra && !props.casm && !props.sierra_text {
            ws.config().ui().warn(
                "Sierra, textual Sierra and CASM lib targets have been disabled, \
                Scarb will not produce anything",
            );
        }

        let target_dir = unit.target_dir(ws);

        let main_crate_ids = collect_main_crate_ids(unit, db);

        let compiler_config = build_compiler_config(db, unit, &main_crate_ids, cached_crates, ws);

        validate_compiler_config(db, &compiler_config, unit, ws);

        let span = trace_span!("compile_sierra");
        let program_artifact = {
            let _guard = span.enter();
            let program_artifact = cairo_lang_compiler::compile_prepared_db_program_artifact(
                db,
                main_crate_ids,
                compiler_config,
            )?;
            Arc::new(program_artifact)
        };

        let span = trace_span!("serialize_sierra_json");
        if props.sierra {
            let _guard = span.enter();
            let target_name = unit.main_component().target_name();
            let target_dir = target_dir.clone();
            // We only clone Arc, not the underlying program, so it's inexpensive.
            let program = program_artifact.clone();
            offloader.offload("output file", move |ws| {
                // Cloning the underlying program is expensive, but we can afford it here,
                // as we are on a dedicated thread anyway.
                let sierra_program: VersionedProgram = program.as_ref().clone().into();
                write_json(
                    &format!("{target_name}.sierra.json"),
                    "output file",
                    &target_dir,
                    ws,
                    &sierra_program,
                )?;
                Ok(())
            });
        }

        let span = trace_span!("serialize_sierra_text");
        if props.sierra_text {
            let _guard = span.enter();
            let target_name = unit.main_component().target_name();
            let target_dir = target_dir.clone();
            // We only clone Arc, not the underlying program, so it's inexpensive.
            let program = program_artifact.clone();
            offloader.offload("output file", move |ws| {
                // Cloning the underlying program is expensive, but we can afford it here,
                // as we are on a dedicated thread anyway.
                let sierra_program: VersionedProgram = program.as_ref().clone().into();
                write_string(
                    &format!("{target_name}.sierra"),
                    "output file",
                    &target_dir,
                    ws,
                    &sierra_program,
                )?;
                Ok(())
            });
        }

        if props.casm {
            let program = &program_artifact.program;

            let span = trace_span!("casm_calc_metadata");
            let metadata = {
                let _guard = span.enter();

                if unit.compiler_config.enable_gas {
                    debug!("calculating Sierra variables");
                    calc_metadata(program, Default::default())
                } else {
                    debug!("calculating Sierra variables with no gas validation");
                    calc_metadata_ap_change_only(program)
                }
                .context("failed calculating Sierra variables")?
            };

            let span = trace_span!("compile_casm");
            let cairo_program = {
                let _guard = span.enter();
                let sierra_to_casm = SierraToCasmConfig {
                    gas_usage_check: unit.compiler_config.enable_gas,
                    max_bytecode_size: usize::MAX,
                };
                cairo_lang_sierra_to_casm::compiler::compile(program, &metadata, sierra_to_casm)?
            };

            let span = trace_span!("serialize_casm");
            {
                let _guard = span.enter();
                write_string(
                    format!("{}.casm", unit.main_component().target_name()).as_str(),
                    "output file",
                    &target_dir,
                    ws,
                    cairo_program,
                )?;
            }
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
    let main_crate_id = unit.main_component().crate_id(db);

    // Generally, lib target compilation should be driven by a certain objective (e.g. cairo-run,
    // test framework, etc.), expressed by the plugin set with executables definition.
    // This does not apply to debug build (expressed by `replace_ids` flag),
    // which is a goal by itself.
    // See starkware-libs/cairo#5440 for more context.
    let executable_plugin = db.crate_macro_plugins(main_crate_id).iter().any(|&plugin| {
        !db.lookup_intern_macro_plugin(plugin)
            .executable_attributes()
            .is_empty()
    });
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
