use anyhow::{ensure, Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::CompilerConfig;
use cairo_lang_defs::ids::NamedLanguageElementId;
use cairo_lang_filesystem::ids::{CrateId, CrateLongId};
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_starknet::compile::compile_prepared_db;
use cairo_lang_starknet::contract::{find_contracts, ContractDeclaration};
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_starknet_classes::contract_class::ContractClass;
use cairo_lang_utils::UpcastMut;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::iter::zip;
use tracing::{debug, trace, trace_span};

use super::contract_selector::ContractSelector;
use crate::compiler::compilers::starknet_contract::contract_selector::GLOB_PATH_SELECTOR;
use crate::compiler::compilers::starknet_contract::validations::check_allowed_libfuncs;
use crate::compiler::compilers::{ensure_gas_enabled, ArtifactsWriter};
use crate::compiler::helpers::{build_compiler_config, collect_main_crate_ids};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, Compiler};
use crate::core::{TargetKind, Workspace};
use crate::internal::serdex::RelativeUtf8PathBuf;
use scarb_ui::Ui;

// TODO(#111): starknet-contract should be implemented as an extension.
pub struct StarknetContractCompiler;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Props {
    pub sierra: bool,
    pub casm: bool,
    pub casm_add_pythonic_hints: bool,
    pub allowed_libfuncs: bool,
    pub allowed_libfuncs_deny: bool,
    pub allowed_libfuncs_list: Option<SerdeListSelector>,
    pub build_external_contracts: Option<Vec<ContractSelector>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            sierra: true,
            casm: false,
            casm_add_pythonic_hints: false,
            allowed_libfuncs: true,
            allowed_libfuncs_deny: false,
            allowed_libfuncs_list: None,
            build_external_contracts: None,
        }
    }
}

// FIXME(#401): Make allowed-libfuncs-list.path relative to current Scarb.toml rather than PWD.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged, rename_all = "kebab-case")]
pub enum SerdeListSelector {
    Name { name: String },
    Path { path: RelativeUtf8PathBuf },
}

impl Compiler for StarknetContractCompiler {
    fn target_kind(&self) -> TargetKind {
        TargetKind::STARKNET_CONTRACT.clone()
    }

    fn compile(
        &self,
        unit: CairoCompilationUnit,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let props: Props = unit.main_component().target_props()?;
        if !props.sierra && !props.casm {
            ws.config().ui().warn(
                "both Sierra and CASM Starknet contract targets have been disabled, \
                Scarb will not produce anything",
            );
        }

        ensure_gas_enabled(db)?;

        if let Some(external_contracts) = props.build_external_contracts.clone() {
            for path in external_contracts.iter() {
                ensure!(path.0.matches(GLOB_PATH_SELECTOR).count() <= 1,
                    "external contract path `{}` has multiple global path selectors, only one '*' selector is allowed",
                    path.0);
            }
        }

        let target_dir = unit.target_dir(ws);

        let main_crate_ids = collect_main_crate_ids(&unit, db);

        let compiler_config = build_compiler_config(db, &unit, &main_crate_ids, ws);

        let contracts = find_project_contracts(
            db.upcast_mut(),
            ws.config().ui(),
            &unit,
            main_crate_ids.clone(),
            props.build_external_contracts.clone(),
        )?;

        let CompiledContracts {
            contract_paths,
            contracts,
            classes,
        } = get_compiled_contracts(contracts, compiler_config, db)?;

        check_allowed_libfuncs(&props, &contracts, &classes, db, &unit, ws)?;

        let casm_classes: Vec<Option<CasmContractClass>> = if props.casm {
            let _ = trace_span!("compile_sierra").enter();
            zip(&contracts, &classes)
                .map(|(decl, class)| -> Result<_> {
                    let contract_name = decl.submodule_id.name(db.upcast_mut());
                    let casm_class = CasmContractClass::from_contract_class(
                        class.clone(),
                        props.casm_add_pythonic_hints,
                        usize::MAX,
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

        let target_name = &unit.main_component().target_name();

        let writer = ArtifactsWriter::new(target_name.clone(), target_dir, props);
        writer.write(contract_paths, &contracts, &classes, &casm_classes, db, ws)?;

        Ok(())
    }
}

pub struct CompiledContracts {
    pub contract_paths: Vec<String>,
    pub contracts: Vec<ContractDeclaration>,
    pub classes: Vec<ContractClass>,
}

pub fn get_compiled_contracts(
    contracts: Vec<ContractDeclaration>,
    compiler_config: CompilerConfig<'_>,
    db: &mut RootDatabase,
) -> Result<CompiledContracts> {
    let contract_paths = contracts
        .iter()
        .map(|decl| decl.module_id().full_path(db.upcast_mut()))
        .collect::<Vec<_>>();
    trace!(contracts = ?contract_paths);

    let classes = {
        let _ = trace_span!("compile_starknet").enter();
        compile_prepared_db(db, &contracts.iter().collect::<Vec<_>>(), compiler_config)?
    };
    Ok(CompiledContracts {
        contract_paths,
        contracts,
        classes,
    })
}

pub fn find_project_contracts(
    mut db: &dyn SemanticGroup,
    ui: Ui,
    unit: &CairoCompilationUnit,
    main_crate_ids: Vec<CrateId>,
    external_contracts: Option<Vec<ContractSelector>>,
) -> Result<Vec<ContractDeclaration>> {
    let internal_contracts = {
        let _ = trace_span!("find_internal_contracts").enter();
        find_contracts(db, &main_crate_ids)
    };

    let external_contracts: Vec<ContractDeclaration> =
        if let Some(external_contracts) = external_contracts {
            let _ = trace_span!("find_external_contracts").enter();
            debug!("external contracts selectors: {:?}", external_contracts);

            let crate_ids = external_contracts
                .iter()
                .map(|selector| selector.package().into())
                .unique()
                .map(|name: SmolStr| {
                    let discriminator = unit
                        .components()
                        .iter()
                        .find(|component| component.package.id.name.to_smol_str() == name)
                        .and_then(|component| component.id.to_discriminator());
                    db.upcast_mut().intern_crate(CrateLongId::Real {
                        name,
                        discriminator,
                    })
                })
                .collect::<Vec<_>>();
            let contracts = find_contracts(db, crate_ids.as_ref());
            let filtered_contracts: Vec<ContractDeclaration> = contracts
                .into_iter()
                .filter(|decl| {
                    let contract_path = decl.module_id().full_path(db.upcast());
                    external_contracts
                        .iter()
                        .any(|selector| contract_matches(selector, contract_path.as_str()))
                })
                .collect();

            let never_matched = external_contracts
                .iter()
                .filter(|selector| {
                    !filtered_contracts.iter().any(|decl| {
                        let contract_path = decl.module_id().full_path(db.upcast());
                        contract_matches(selector, contract_path.as_str())
                    })
                })
                .collect_vec();
            if !never_matched.is_empty() {
                let never_matched = never_matched
                    .iter()
                    .map(|selector| selector.full_path())
                    .collect_vec()
                    .join("`, `");
                ui.warn(format!(
                    "external contracts not found for selectors: `{never_matched}`"
                ));
            }

            filtered_contracts
        } else {
            debug!("no external contracts selected");
            Vec::new()
        };

    Ok(internal_contracts
        .into_iter()
        .chain(external_contracts)
        .collect())
}

fn contract_matches(selector: &ContractSelector, contract_path: &str) -> bool {
    if selector.is_wildcard() {
        contract_path.starts_with(&selector.partial_path())
    } else {
        contract_path == selector.full_path()
    }
}
