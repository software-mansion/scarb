use anyhow::{ensure, Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::CompilerConfig;
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::{ModuleId, NamedLanguageElementId};
use cairo_lang_filesystem::ids::{CrateId, CrateLongId};
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_semantic::items::us::SemanticUseEx;
use cairo_lang_semantic::items::visibility::Visibility;
use cairo_lang_semantic::resolve::ResolvedGenericItem::Module;
use cairo_lang_starknet::compile::compile_prepared_db;
use cairo_lang_starknet::contract::{find_contracts, module_contract, ContractDeclaration};
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_starknet_classes::contract_class::ContractClass;
use cairo_lang_syntax::node::ast::OptionAliasClause;
use cairo_lang_syntax::node::TypedSyntaxNode;
use cairo_lang_utils::UpcastMut;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::collections::HashSet;
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
            let span = trace_span!("compile_sierra");
            let _guard = span.enter();

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

    let span = trace_span!("compile_starknet");
    let classes = {
        let _guard = span.enter();
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
    let span = trace_span!("find_internal_contracts");
    let internal_contracts = {
        let _guard = span.enter();
        find_contracts(db, &main_crate_ids)
    };

    let span = trace_span!("find_external_contracts");
    let external_contracts: Vec<ContractDeclaration> = if let Some(external_contracts) =
        external_contracts
    {
        let _guard = span.enter();
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
        let mut filtered_contracts: Vec<ContractDeclaration> = contracts
            .into_iter()
            .filter(|decl| {
                let contract_path = decl.module_id().full_path(db.upcast());
                external_contracts
                    .iter()
                    .any(|selector| contract_matches(selector, contract_path.as_str()))
            })
            .collect();

        let mut matched_selectors: HashSet<ContractSelector> = external_contracts
            .iter()
            .filter(|selector| {
                filtered_contracts.iter().any(|decl| {
                    let contract_path = decl.module_id().full_path(db.upcast());
                    contract_matches(selector, contract_path.as_str())
                })
            })
            .cloned()
            .collect();

        // Find selected reexports.
        for crate_id in crate_ids {
            let modules = db.crate_modules(crate_id);
            for module_id in modules.iter() {
                let Ok(module_uses) = db.module_uses(*module_id) else {
                    continue;
                };
                let module_with_reexport = module_id.full_path(db.upcast());
                let matched_contracts = module_uses
                    .iter()
                    .filter_map(|(use_id, use_path)| {
                        let use_alias = match use_path.alias_clause(db.upcast()) {
                            OptionAliasClause::Empty(_) => None,
                            OptionAliasClause::AliasClause(alias_clause) => Some(
                                alias_clause
                                    .alias(db.upcast())
                                    .as_syntax_node()
                                    .get_text_without_trivia(db.upcast()),
                            ),
                        };
                        let visibility = db
                            .module_item_info_by_name(*module_id, use_id.name(db.upcast()))
                            .ok()??
                            .visibility;
                        if visibility == Visibility::Public {
                            Some((db.use_resolved_item(*use_id).ok()?, use_alias))
                        } else {
                            None
                        }
                    })
                    .filter_map(|(use_item, use_alias)| match use_item {
                        Module(module_id) => {
                            module_id.name(db.upcast());
                            Some((module_id, use_alias))
                        }
                        _ => None,
                    })
                    .flat_map(|(module_id, use_alias)| {
                        let exported_module_path = module_id.full_path(db.upcast());
                        let exported_module_name =
                            use_alias.unwrap_or_else(|| module_id.name(db.upcast()).to_string());
                        let mut submodules = Vec::new();
                        collect_modules_under(db.upcast(), &mut submodules, module_id);
                        let found_contracts = submodules
                            .iter()
                            .filter_map(|module_id| {
                                let contract = module_contract(db, *module_id)?;
                                let contract_path = contract.module_id().full_path(db.upcast());
                                let exported_contract_path =
                                    contract_path.replace(&exported_module_path, "");
                                let exported_contract_path = format!(
                                    "{}::{exported_module_name}{exported_contract_path}",
                                    &module_with_reexport
                                );
                                let selectors_used = external_contracts
                                    .iter()
                                    .filter(|selector| {
                                        contract_matches(selector, exported_contract_path.as_str())
                                    })
                                    .map(|c| (*c).clone())
                                    .collect_vec();
                                let any_matched = !selectors_used.is_empty();
                                matched_selectors.extend(selectors_used);
                                any_matched.then_some(contract)
                            })
                            .collect_vec();
                        found_contracts
                    })
                    .collect_vec();
                filtered_contracts.extend(matched_contracts);
            }
        }

        let never_matched = external_contracts
            .iter()
            .filter(|selector| !matched_selectors.contains(*selector))
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

fn collect_modules_under(db: &dyn DefsGroup, modules: &mut Vec<ModuleId>, module_id: ModuleId) {
    modules.push(module_id);
    if let Ok(submodule_ids) = db.module_submodules_ids(module_id) {
        for submodule_module_id in submodule_ids.iter().copied() {
            collect_modules_under(db, modules, ModuleId::Submodule(submodule_module_id));
        }
    }
}

fn contract_matches(selector: &ContractSelector, contract_path: &str) -> bool {
    if selector.is_wildcard() {
        contract_path.starts_with(&selector.partial_path())
    } else {
        contract_path == selector.full_path()
    }
}
