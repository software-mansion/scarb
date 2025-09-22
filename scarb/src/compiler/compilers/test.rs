use anyhow::Result;
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_filesystem::ids::{CrateId, CrateLongId};
use cairo_lang_sierra::program::VersionedProgram;
use cairo_lang_starknet::compile::compile_prepared_db;
use cairo_lang_starknet::contract::ContractDeclaration;
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_test_plugin::{TestsCompilationConfig, compile_test_prepared_db};
use cairo_lang_utils::Intern;
use itertools::Itertools;
use tracing::{trace, trace_span};

use crate::compiler::compilers::starknet_contract::Props as StarknetContractProps;
use crate::compiler::compilers::{
    Artifacts, ArtifactsWriter, ContractSelector, ensure_gas_enabled, find_project_contracts,
};
use crate::compiler::helpers::{build_compiler_config, collect_main_crate_ids, write_json};
use crate::compiler::incremental::IncrementalContext;
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, Compiler};
use crate::core::{PackageName, SourceId, TargetKind, TestTargetProps, Workspace};
use crate::flock::Filesystem;
use crate::internal::offloader::Offloader;

pub struct TestCompiler;

impl Compiler for TestCompiler {
    fn target_kind(&self) -> TargetKind {
        TargetKind::TEST.clone()
    }

    fn compile(
        &self,
        unit: &CairoCompilationUnit,
        ctx: &IncrementalContext,
        offloader: &Offloader<'_>,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let target_dir = unit.target_dir(ws);
        let build_external_contracts = external_contracts_selectors(unit)?;

        let test_crate_ids = collect_main_crate_ids(unit, db);
        // Search for all contracts in deps specified with `build-external-contracts`.
        let all_crate_ids =
            get_contract_crate_ids(&build_external_contracts, test_crate_ids.clone(), unit, db);

        let starknet = unit.cairo_plugins.iter().any(|plugin| {
            plugin.package.id.name == PackageName::STARKNET
                && plugin.package.id.source_id == SourceId::for_std()
        });

        let contracts = if starknet {
            find_project_contracts(
                db,
                ws.config().ui(),
                unit,
                test_crate_ids.clone(),
                build_external_contracts.clone(),
            )?
        } else {
            Vec::new()
        };

        let diagnostics_reporter =
            build_compiler_config(db, unit, &test_crate_ids, ctx, ws).diagnostics_reporter;

        let span = trace_span!("compile_test");
        let test_compilation = {
            let _guard = span.enter();
            let config = TestsCompilationConfig {
                starknet,
                add_statements_functions: unit
                    .compiler_config
                    .unstable_add_statements_functions_debug_info,
                add_statements_code_locations: unit
                    .compiler_config
                    .unstable_add_statements_code_locations_debug_info,
                contract_crate_ids: starknet.then_some(&all_crate_ids),
                executable_crate_ids: None,
                contract_declarations: starknet.then_some(contracts.clone()),
            };
            compile_test_prepared_db(
                db,
                config,
                test_crate_ids
                    .clone()
                    .into_iter()
                    .map(|c| c.long(db).clone().into_crate_input(db))
                    .collect_vec(),
                diagnostics_reporter,
            )?
        };

        let span = trace_span!("serialize_test");
        {
            let _guard = span.enter();
            {
                let target_name = unit.main_component().target_name();
                let target_dir = target_dir.clone();
                offloader.offload("output file", move |ws| {
                    // Cloning the underlying program is expensive, but we can afford it here,
                    // as we are on a dedicated thread anyway.
                    let sierra_program: VersionedProgram = test_compilation.sierra_program.into();
                    write_json(
                        &format!("{target_name}.test.sierra.json"),
                        "output file",
                        &target_dir,
                        ws,
                        &sierra_program,
                    )?;
                    Ok(())
                });
            }

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
            compile_contracts(
                ContractsCompilationArgs {
                    main_crate_ids: test_crate_ids,
                    contracts,
                    build_external_contracts,
                },
                target_dir,
                unit,
                offloader,
                ctx,
                db,
                ws,
            )?;
        }

        Ok(())
    }
}

struct ContractsCompilationArgs<'db> {
    main_crate_ids: Vec<CrateId<'db>>,
    contracts: Vec<ContractDeclaration<'db>>,
    build_external_contracts: Option<Vec<ContractSelector>>,
}

fn compile_contracts<'db>(
    args: ContractsCompilationArgs<'db>,
    target_dir: Filesystem,
    unit: &CairoCompilationUnit,
    offloader: &Offloader<'_>,
    ctx: &IncrementalContext,
    db: &'db RootDatabase,
    ws: &Workspace<'_>,
) -> Result<()> {
    let ContractsCompilationArgs {
        main_crate_ids,
        contracts,
        build_external_contracts,
    } = args;
    ensure_gas_enabled(db)?;
    let target_name = unit.main_component().target_name();
    let props = StarknetContractProps {
        build_external_contracts,
        ..StarknetContractProps::default()
    };
    let mut compiler_config = build_compiler_config(db, unit, &main_crate_ids, ctx, ws);
    // We already did check the Db for diagnostics when compiling tests, so we can ignore them here.
    compiler_config.diagnostics_reporter = DiagnosticsReporter::ignoring()
        .allow_warnings()
        .with_crates(&[]);
    let contract_paths = contracts
        .iter()
        .map(|decl| decl.module_id().full_path(db))
        .collect::<Vec<_>>();
    trace!(contracts = ?contract_paths);
    let span = trace_span!("compile_starknet");
    let classes = {
        let _guard = span.enter();
        compile_prepared_db(db, &contracts.iter().collect::<Vec<_>>(), compiler_config)?
    };
    let writer = ArtifactsWriter::new(target_name.clone(), target_dir, props)
        .with_extension_prefix("test".to_string());
    let casm_classes: Vec<Option<CasmContractClass>> = classes.iter().map(|_| None).collect();
    writer.write(
        Artifacts {
            contract_paths,
            contracts,
            classes,
            casm_classes,
        },
        offloader,
        db,
        ws,
    )?;
    Ok(())
}

fn external_contracts_selectors(
    unit: &CairoCompilationUnit,
) -> Result<Option<Vec<ContractSelector>>> {
    let test_props: TestTargetProps = unit.main_component().targets.target_props()?;
    Ok(test_props
        .build_external_contracts
        .map(|contracts| contracts.into_iter().map(ContractSelector).collect_vec()))
}

fn get_contract_crate_ids<'db>(
    build_external_contracts: &Option<Vec<ContractSelector>>,
    test_crate_ids: Vec<CrateId<'db>>,
    unit: &CairoCompilationUnit,
    db: &'db RootDatabase,
) -> Vec<CrateId<'db>> {
    let mut all_crate_ids = build_external_contracts
        .as_ref()
        .map(|external_contracts| {
            external_contracts
                .iter()
                .map(|selector| selector.package())
                .sorted()
                .unique()
                .map(|package_name| {
                    let discriminator = unit
                        .components()
                        .iter()
                        .find(|component| component.package.id.name == package_name)
                        .and_then(|component| component.id.to_discriminator());
                    let name = package_name.to_string();
                    CrateLongId::Real {
                        name,
                        discriminator,
                    }
                    .intern(db)
                })
                .collect_vec()
        })
        .unwrap_or_default();
    all_crate_ids.extend(test_crate_ids);
    all_crate_ids
}
