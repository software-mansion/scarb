use anyhow::Result;
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_filesystem::ids::CrateId;
use cairo_lang_sierra::program::VersionedProgram;
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_test_plugin::{compile_test_prepared_db, TestsCompilationConfig};
use itertools::Itertools;
use tracing::trace_span;

use crate::compiler::compilers::starknet_contract::Props as StarknetContractProps;
use crate::compiler::compilers::{
    ensure_gas_enabled, get_compiled_contracts, ArtifactsWriter, CompiledContracts,
    ContractSelector,
};
use crate::compiler::helpers::{
    build_compiler_config, collect_all_crate_ids, collect_main_crate_ids, write_json,
};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, Compiler};
use crate::core::{PackageName, SourceId, TargetKind, TestTargetProps, Workspace};
use crate::flock::Filesystem;

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
        let all_crate_ids = collect_all_crate_ids(&unit, db);
        let starknet = unit.cairo_plugins.iter().any(|plugin| {
            plugin.package.id.name == PackageName::STARKNET
                && plugin.package.id.source_id == SourceId::for_std()
        });

        let diagnostics_reporter =
            build_compiler_config(db, &unit, &test_crate_ids, ws).diagnostics_reporter;

        let test_compilation = {
            let _ = trace_span!("compile_test").enter();
            let config = TestsCompilationConfig {
                starknet,
                add_statements_functions: unit
                    .compiler_config
                    .unstable_add_statements_functions_debug_info,
                add_statements_code_locations: unit
                    .compiler_config
                    .unstable_add_statements_code_locations_debug_info,
            };
            compile_test_prepared_db(
                db,
                config,
                all_crate_ids,
                test_crate_ids.clone(),
                diagnostics_reporter,
            )?
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

        if starknet {
            // Note: this will only search for contracts in the main CU component and
            // `build-external-contracts`. It will not collect contracts from all dependencies.
            compile_contracts(test_crate_ids, target_dir, unit, db, ws)?;
        }

        Ok(())
    }
}

fn compile_contracts(
    main_crate_ids: Vec<CrateId>,
    target_dir: Filesystem,
    unit: CairoCompilationUnit,
    db: &mut RootDatabase,
    ws: &Workspace<'_>,
) -> Result<()> {
    ensure_gas_enabled(db)?;
    let target_name = unit.main_component().target_name();
    let test_props: TestTargetProps = unit.main_component().target_props()?;
    let external_contracts = test_props
        .build_external_contracts
        .map(|contracts| contracts.into_iter().map(ContractSelector).collect_vec());
    let props = StarknetContractProps {
        build_external_contracts: external_contracts,
        ..StarknetContractProps::default()
    };
    let compiler_config = build_compiler_config(db, &unit, &main_crate_ids, ws);
    let CompiledContracts {
        contract_paths,
        contracts,
        classes,
    } = get_compiled_contracts(
        main_crate_ids,
        props.build_external_contracts.clone(),
        compiler_config,
        db,
        ws,
    )?;
    let writer = ArtifactsWriter::new(target_name.clone(), target_dir, props)
        .with_extension_prefix("test".to_string());
    let casm_classes: Vec<Option<CasmContractClass>> = classes.iter().map(|_| None).collect();
    writer.write(contract_paths, &contracts, &classes, &casm_classes, db, ws)?;
    Ok(())
}
