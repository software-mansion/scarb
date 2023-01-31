use std::iter::zip;
use std::ops::DerefMut;

use anyhow::{ensure, Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_starknet::contract::find_contracts;
use cairo_lang_starknet::contract_class::compile_prepared_db;
use cairo_lang_starknet::db::StarknetRootDatabaseBuilderEx;
use cairo_lang_utils::Upcast;
use tracing::{span, trace, Level};

use crate::compiler::targets::lib::{
    build_compiler_config, build_project_config, collect_main_crate_ids,
};
use crate::compiler::CompilationUnit;
use crate::core::{ExternalTargetKind, Workspace};

#[tracing::instrument(level = "trace", skip_all, fields(unit = unit.name()))]
pub fn compile_contract(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let props = unit.target.kind.downcast::<ExternalTargetKind>();
    ensure!(
        props.params.is_empty(),
        "target `{}` does not accept any parameters",
        props.kind_name
    );

    let target_dir = unit.profile.target_dir(ws.config());

    let mut db = RootDatabase::builder()
        .with_project_config(build_project_config(&unit)?)
        .with_starknet()
        .build()?;

    let compiler_config = build_compiler_config(ws);

    let main_crate_ids = collect_main_crate_ids(&unit, &db);

    let contracts = {
        let _ = span!(Level::TRACE, "find_contracts").enter();
        find_contracts(&db, &main_crate_ids)
    };

    trace!(
        contracts = ?contracts
            .iter()
            .map(|decl| decl.module_id().full_path(db.upcast()))
            .collect::<Vec<_>>()
    );

    let contracts = contracts.iter().collect::<Vec<_>>();

    let classes = {
        let _ = span!(Level::TRACE, "compile_starknet").enter();
        compile_prepared_db(&mut db, &contracts, compiler_config)?
    };

    for (decl, class) in zip(contracts, classes) {
        let target_name = &unit.target.name;
        let contract_name = decl.submodule_id.name(db.upcast());
        let mut file = target_dir.open_rw(
            format!("{target_name}_{contract_name}.json"),
            "output file",
            ws.config(),
        )?;
        serde_json::to_writer_pretty(file.deref_mut(), &class)
            .with_context(|| format!("Failed to serialize contract: {contract_name}"))?;
    }

    Ok(())
}
