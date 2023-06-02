use std::iter::zip;

use anyhow::{Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_starknet::casm_contract_class::CasmContractClass;
use cairo_lang_starknet::contract::find_contracts;
use cairo_lang_starknet::contract_class::compile_prepared_db;
use cairo_lang_utils::UpcastMut;
use itertools::{izip, Itertools};
use serde::{Deserialize, Serialize};
use tracing::{trace, trace_span};

use crate::compiler::helpers::{build_compiler_config, collect_main_crate_ids};
use crate::compiler::{CompilationUnit, Compiler};
use crate::core::{PackageName, Workspace};
use crate::flock::Filesystem;
use crate::internal::stable_hash::short_hash;

// TODO(#111): starknet-contract should be implemented as an extension.
pub struct StarknetContractCompiler;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Props {
    pub sierra: bool,
    pub casm: bool,
    pub casm_add_pythonic_hints: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            sierra: true,
            casm: false,
            casm_add_pythonic_hints: false,
        }
    }
}

#[derive(Debug, Serialize)]
struct StarknetArtifacts {
    version: usize,
    contracts: Vec<ContractArtifacts>,
}

impl Default for StarknetArtifacts {
    fn default() -> Self {
        Self {
            version: 1,
            contracts: Vec::new(),
        }
    }
}

impl StarknetArtifacts {
    fn finish(&mut self) {
        assert!(
            self.contracts.iter().map(|it| &it.id).all_unique(),
            "Artifacts IDs must be unique."
        );

        self.contracts.sort_unstable_by_key(|it| it.id.clone());
    }
}

#[derive(Debug, Serialize)]
struct ContractArtifacts {
    id: String,
    package_name: PackageName,
    contract_name: String,
    artifacts: ContractArtifact,
}

impl ContractArtifacts {
    fn new(package_name: &PackageName, contract_name: &str) -> Self {
        Self {
            id: short_hash((&package_name, &contract_name)),
            package_name: package_name.clone(),
            contract_name: contract_name.to_owned(),
            artifacts: ContractArtifact::default(),
        }
    }
}

#[derive(Debug, Default, Serialize)]
struct ContractArtifact {
    sierra: Option<String>,
    casm: Option<String>,
}

impl Compiler for StarknetContractCompiler {
    fn target_kind(&self) -> &str {
        "starknet-contract"
    }

    fn compile(
        &self,
        unit: CompilationUnit,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let props: Props = unit.target().props()?;
        if !props.sierra && !props.casm {
            ws.config().ui().warn(
                "both Sierra and CASM Starknet contract targets have been disabled, \
                Scarb will not produce anything",
            );
        }

        let target_dir = unit.target_dir(ws.config());

        let compiler_config = build_compiler_config(&unit, ws);

        let main_crate_ids = collect_main_crate_ids(&unit, db);

        let contracts = {
            let _ = trace_span!("find_contracts").enter();
            find_contracts(db.upcast_mut(), &main_crate_ids)
        };

        trace!(
            contracts = ?contracts
                .iter()
                .map(|decl| decl.module_id().full_path(db.upcast_mut()))
                .collect::<Vec<_>>()
        );

        let contracts = contracts.iter().collect::<Vec<_>>();

        let classes = {
            let _ = trace_span!("compile_starknet").enter();
            compile_prepared_db(db, &contracts, compiler_config)?
        };

        let casm_classes: Vec<Option<CasmContractClass>> = if props.casm {
            let _ = trace_span!("compile_sierra").enter();
            zip(&contracts, &classes)
                .map(|(decl, class)| -> Result<_> {
                    let contract_name = decl.submodule_id.name(db.upcast_mut());
                    let casm_class = CasmContractClass::from_contract_class(
                        class.clone(),
                        props.casm_add_pythonic_hints,
                    )
                    .with_context(|| {
                        format!("{contract_name}: failed to compile Sierra contract to CASM")
                    })?;
                    Ok(Some(casm_class))
                })
                .try_collect()?
        } else {
            classes.iter().map(|_| None).collect()
        };

        let mut artifacts = StarknetArtifacts::default();

        for (decl, class, casm_class) in izip!(contracts, classes, casm_classes) {
            let target_name = &unit.target().name;
            let contract_name = decl.submodule_id.name(db.upcast_mut());
            let file_stem = format!("{target_name}_{contract_name}");

            let mut artifact = ContractArtifacts::new(&unit.main_package_id.name, &contract_name);

            if props.sierra {
                let file_name = format!("{file_stem}.sierra.json");
                write_json(&file_name, "output file", &target_dir, ws, &class)?;
                artifact.artifacts.sierra = Some(file_name);
            }

            // if props.casm
            if let Some(casm_class) = casm_class {
                let file_name = format!("{file_stem}.casm.json");
                write_json(&file_name, "output file", &target_dir, ws, &casm_class)?;
                artifact.artifacts.casm = Some(file_name);
            }

            artifacts.contracts.push(artifact);
        }

        artifacts.finish();

        write_json(
            &format!("{}.starknet_artifacts.json", unit.main_package_id.name),
            "starknet artifacts file",
            &target_dir,
            ws,
            &artifacts,
        )?;

        Ok(())
    }
}

fn write_json(
    file_name: &str,
    description: &str,
    target_dir: &Filesystem<'_>,
    ws: &Workspace<'_>,
    value: impl Serialize,
) -> Result<()> {
    let mut file = target_dir.open_rw(file_name, description, ws.config())?;
    serde_json::to_writer(&mut *file, &value)
        .with_context(|| format!("failed to serialize {file_name}"))
}
