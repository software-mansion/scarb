use anyhow::{Context, Result, bail, ensure};
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_compiler::{CompilerConfig, ensure_diagnostics};
use cairo_lang_semantic::items::constant::ConstValueId;
use cairo_lang_sierra_generator::db::SierraGenGroup;
use cairo_lang_starknet::compile::compile_prepared_db;
use cairo_lang_starknet::contract::ContractDeclaration;
use cairo_lang_starknet_classes::contract_class::ContractClass;
use cairo_lang_utils::CloneableDatabase;
use cairo_lang_utils::bigint::BigIntAsHex;
use itertools::Itertools;
use scarb_ui::Ui;
use starknet_core::types::contract::SierraClass;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tracing::{trace, trace_span};

use super::compiler::find_project_contracts_silent;
use super::contract_selector::ContractSelector;
use crate::compiler::CairoCompilationUnit;
use crate::compiler::helpers::{build_compiler_config, collect_main_crate_ids};
use crate::compiler::incremental::IncrementalContext;
use crate::core::Workspace;

pub(super) struct ForwardingCompilation {
    pub contract_paths: Vec<String>,
    pub classes: Vec<ContractClass>,
}

struct DiscoveredContract {
    deps: HashSet<String>,
    class: ContractClass,
}

pub(super) fn compile_with_forwarding(
    db: &mut dyn CloneableDatabase,
    unit: &CairoCompilationUnit,
    ctx: &Arc<IncrementalContext>,
    ws: &Workspace<'_>,
    external_contracts: Option<Vec<ContractSelector>>,
) -> Result<ForwardingCompilation> {
    let main_crate_ids = collect_main_crate_ids(unit, db);
    let contract_paths = {
        let contracts = find_project_contracts_silent(
            db,
            ws.config().ui(),
            unit,
            main_crate_ids.clone(),
            external_contracts.clone(),
        )?;
        contracts
            .iter()
            .map(|decl| decl.module_id().full_path(db).to_string())
            .collect_vec()
    };
    trace!(contracts = ?contract_paths);

    {
        let mut diagnostics_config =
            build_compiler_config(db, unit, &main_crate_ids, ctx, ctx.warning_collector(), ws);
        ensure_diagnostics(db, &mut diagnostics_config.diagnostics_reporter)?;
    }

    let span = trace_span!("compile_starknet");
    let classes = {
        let _guard = span.enter();
        compile_contracts_in_forwarding_order(
            db,
            unit,
            ws.config().ui(),
            external_contracts.clone(),
            &contract_paths,
        )?
    };
    db.set_external_const_provider(None);

    Ok(ForwardingCompilation {
        contract_paths,
        classes,
    })
}

fn compile_contracts_in_forwarding_order<'db>(
    db: &'db mut dyn CloneableDatabase,
    unit: &CairoCompilationUnit,
    ui: Ui,
    external_contracts: Option<Vec<ContractSelector>>,
    contract_paths: &[String],
) -> Result<Vec<ContractClass>> {
    // First compile each contract with dummy externs and keep artifacts that did not consume any.
    // Then repeatedly recompile contracts whose deps already have final declaration hashes.
    // Each final artifact contributes its Starknet declaration hash to later dependent compilations.
    let target_keys = contract_paths
        .iter()
        .map(|contract_path| {
            (
                class_hash_provider_key(contract_path),
                contract_path.clone(),
            )
        })
        .collect::<HashMap<_, _>>();

    let mut discovered = HashMap::<String, DiscoveredContract>::new();
    for contract_path in contract_paths {
        let key = class_hash_provider_key(contract_path);
        // Record unresolved externs without making this discovery pass depend on real hashes.
        let recorded = Arc::new(Mutex::new(HashSet::<String>::new()));
        install_recording_provider(db, recorded.clone());
        let class = {
            let contract = find_project_contract_by_path(
                db,
                ui.clone(),
                unit,
                external_contracts.clone(),
                contract_path,
            )?;
            compile_prepared_db(db, &[&contract], silent_compiler_config(unit))?
                .into_iter()
                .exactly_one()?
        };
        let recorded = recorded
            .lock()
            .expect("recording provider mutex poisoned")
            .clone();
        let unknown = recorded
            .iter()
            .filter(|path| !target_keys.contains_key(*path))
            .sorted()
            .cloned()
            .collect_vec();
        ensure!(
            unknown.is_empty(),
            "contract `{}` forwards to contract(s) that are not included in this build: {}. \
             Add them with `build-external-contracts` or fix the forwarding target.",
            target_keys.get(&key).expect("key exists"),
            unknown.join(", ")
        );
        discovered.insert(
            key,
            DiscoveredContract {
                deps: recorded,
                class,
            },
        );
    }

    let self_injecting = discovered
        .iter()
        .filter_map(|(key, discovered)| discovered.deps.contains(key).then_some(key.clone()))
        .collect::<HashSet<_>>();

    for target in &self_injecting {
        let users = discovered
            .iter()
            .filter(|(source, discovered)| *source != target && discovered.deps.contains(target))
            .map(|(source, _)| target_keys.get(source).expect("key exists").clone())
            .sorted()
            .collect_vec();
        ensure!(
            users.is_empty(),
            "contract `{}` uses its own generated class hash and is also used as a forwarding \
             target by: {}. Forwarding to such a contract is unsound because its embedded self-hash \
             is an approximation.",
            target_keys.get(target).expect("key exists"),
            users.join(", ")
        );
    }

    let mut known = HashMap::<String, BigIntAsHex>::new();
    let mut classes_by_key = HashMap::<String, ContractClass>::new();
    for (key, discovered) in discovered.iter() {
        if discovered.deps.is_empty() {
            // Non-forwarding contracts keep their first-pass artifact.
            known.insert(
                key.clone(),
                BigIntAsHex::from(starknet_class_hash(&discovered.class)?.to_biguint()),
            );
            classes_by_key.insert(key.clone(), discovered.class.clone());
        }
    }
    let mut remaining = contract_paths
        .iter()
        .map(|contract| class_hash_provider_key(contract))
        .filter(|key| !classes_by_key.contains_key(key))
        .collect::<HashSet<_>>();

    while !remaining.is_empty() {
        // Only contracts whose forwarded class hashes are known can be compiled in this pass.
        let ready = remaining
            .iter()
            .filter(|key| {
                discovered
                    .get(*key)
                    .expect("dependencies exist")
                    .deps
                    .iter()
                    .all(|dep| dep == *key || known.contains_key(dep))
            })
            .cloned()
            .sorted()
            .collect_vec();

        if ready.is_empty() {
            let cycle_members = remaining
                .iter()
                .map(|key| target_keys.get(key).expect("key exists").clone())
                .sorted()
                .join(", ");
            bail!("forwarding cycle detected among contracts: {cycle_members}");
        }

        for key in ready {
            let contract_path = target_keys
                .get(&key)
                .expect("ready key must refer to a contract");

            let class = if self_injecting.contains(&key) {
                let deps = &discovered.get(&key).expect("contract exists").deps;
                let stub = if deps.len() == 1 && deps.contains(&key) {
                    discovered.get(&key).expect("contract exists").class.clone()
                } else {
                    let mut provider_values = known.clone();
                    provider_values.insert(key.clone(), BigIntAsHex::from(0));
                    compile_with_resolved_provider(
                        db,
                        unit,
                        ui.clone(),
                        external_contracts.clone(),
                        contract_path,
                        provider_values,
                    )?
                };

                let mut provider_values = known.clone();
                provider_values.insert(
                    key.clone(),
                    BigIntAsHex::from(starknet_class_hash(&stub)?.to_biguint()),
                );
                compile_with_resolved_provider(
                    db,
                    unit,
                    ui.clone(),
                    external_contracts.clone(),
                    contract_path,
                    provider_values,
                )?
            } else {
                compile_with_resolved_provider(
                    db,
                    unit,
                    ui.clone(),
                    external_contracts.clone(),
                    contract_path,
                    known.clone(),
                )?
            };

            // Dependents must receive the declaration hash that Starknet accepts.
            known.insert(
                key.clone(),
                BigIntAsHex::from(starknet_class_hash(&class)?.to_biguint()),
            );
            classes_by_key.insert(key.clone(), class);
            remaining.remove(&key);
        }
    }

    contract_paths
        .iter()
        .map(|contract_path| {
            let key = class_hash_provider_key(contract_path);
            classes_by_key
                .remove(&key)
                .with_context(|| format!("missing compiled class for `{contract_path}`"))
        })
        .collect()
}

fn starknet_class_hash(class: &ContractClass) -> Result<starknet_core::types::Felt> {
    let class: SierraClass = serde_json::from_value(serde_json::to_value(class)?)
        .context("failed to convert Starknet contract class for class hash computation")?;
    Ok(class.class_hash()?)
}

fn compile_with_resolved_provider<'db>(
    db: &'db mut dyn CloneableDatabase,
    unit: &CairoCompilationUnit,
    ui: Ui,
    external_contracts: Option<Vec<ContractSelector>>,
    contract_path: &str,
    known: HashMap<String, BigIntAsHex>,
) -> Result<ContractClass> {
    let missing = Arc::new(Mutex::new(HashSet::<String>::new()));
    install_resolving_provider(db, known, missing.clone());
    let class = {
        let contract =
            find_project_contract_by_path(db, ui, unit, external_contracts, contract_path)?;
        compile_prepared_db(db, &[&contract], silent_compiler_config(unit))?
            .into_iter()
            .exactly_one()?
    };
    let missing = missing
        .lock()
        .expect("missing provider keys mutex poisoned")
        .iter()
        .sorted()
        .cloned()
        .collect_vec();
    ensure!(
        missing.is_empty(),
        "contract `{}` requested unresolved forwarding class hash(es): {}",
        contract_path,
        missing.join(", ")
    );
    Ok(class)
}

fn find_project_contract_by_path<'db>(
    db: &'db dyn CloneableDatabase,
    ui: Ui,
    unit: &CairoCompilationUnit,
    external_contracts: Option<Vec<ContractSelector>>,
    contract_path: &str,
) -> Result<ContractDeclaration<'db>> {
    find_project_contracts_silent(
        db,
        ui,
        unit,
        collect_main_crate_ids(unit, db),
        external_contracts,
    )?
    .into_iter()
    .find(|contract| contract.module_id().full_path(db) == contract_path)
    .with_context(|| format!("failed to find contract `{contract_path}`"))
}

fn class_hash_provider_key(contract_path: &str) -> String {
    format!("{contract_path}::__class_hash__::__externally_provided_const__")
}

fn install_recording_provider(
    db: &mut dyn CloneableDatabase,
    recorded: Arc<Mutex<HashSet<String>>>,
) {
    db.set_external_const_provider(Some(Arc::new(move |db, full_path, ty| {
        recorded
            .lock()
            .expect("recording provider mutex poisoned")
            .insert(full_path.to_owned());
        Ok(ConstValueId::from_int(db, ty, &BigIntAsHex::from(0).value))
    })));
}

fn install_resolving_provider(
    db: &mut dyn CloneableDatabase,
    known: HashMap<String, BigIntAsHex>,
    missing: Arc<Mutex<HashSet<String>>>,
) {
    db.set_external_const_provider(Some(Arc::new(move |db, full_path, ty| {
        if let Some(value) = known.get(full_path) {
            Ok(ConstValueId::from_int(db, ty, &value.value))
        } else {
            missing
                .lock()
                .expect("missing provider keys mutex poisoned")
                .insert(full_path.to_owned());
            Ok(ConstValueId::from_int(db, ty, &BigIntAsHex::from(0).value))
        }
    })));
}

fn silent_compiler_config(unit: &CairoCompilationUnit) -> CompilerConfig<'static> {
    CompilerConfig {
        diagnostics_reporter: DiagnosticsReporter::ignoring()
            .allow_warnings()
            .with_crates(&[]),
        replace_ids: unit.compiler_config.sierra_replace_ids,
        add_statements_functions: unit.compiler_config.add_statements_functions_debug_info,
        add_statements_code_locations: unit
            .compiler_config
            .add_statements_code_locations_debug_info,
        add_functions_debug_info: unit.compiler_config.add_functions_debug_info,
        add_type_names: unit.compiler_config.add_types_debug_info,
    }
}
