use crate::compiler::compilers::{Props, SerdeListSelector};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes};
use crate::core::{Utf8PathWorkspaceExt, Workspace};
use anyhow::{Context, bail, ensure};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_defs::ids::NamedLanguageElementId;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::flag::Flag;
use cairo_lang_filesystem::ids::{FlagId, FlagLongId};
use cairo_lang_starknet::contract::ContractDeclaration;
use cairo_lang_starknet_classes::allowed_libfuncs::{
    AllowedLibfuncsError, BUILTIN_EXPERIMENTAL_LIBFUNCS_LIST, ListSelector,
};
use cairo_lang_starknet_classes::contract_class::ContractClass;
use indoc::{formatdoc, writedoc};
use std::fmt::Write;
use std::iter::zip;
use tracing::debug;

const AUTO_WITHDRAW_GAS_FLAG: &str = "add_withdraw_gas";

pub fn ensure_gas_enabled(db: &mut RootDatabase) -> anyhow::Result<()> {
    let flag = FlagId::new(db, FlagLongId(AUTO_WITHDRAW_GAS_FLAG.into()));
    let flag = db.get_flag(flag);
    ensure!(
        flag.map(|f| matches!(*f, Flag::AddWithdrawGas(true)))
            .unwrap_or(false),
        "the target starknet contract compilation requires gas to be enabled"
    );
    Ok(())
}

pub fn check_allowed_libfuncs<'db>(
    props: &Props,
    contracts: &[ContractDeclaration<'db>],
    classes: &[ContractClass],
    db: &'db RootDatabase,
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
) -> anyhow::Result<()> {
    if !props.allowed_libfuncs {
        debug!("allowed libfuncs checking disabled by target props");
        return Ok(());
    }

    let list_selector = match &props.allowed_libfuncs_list {
        Some(SerdeListSelector::Name { name }) => ListSelector::ListName(name.clone()),
        Some(SerdeListSelector::Path { path }) => {
            let path = path.relative_to_file(unit.main_component().package.manifest_path())?;
            ListSelector::ListFile(path.into_string())
        }
        None => Default::default(),
    };

    let mut found_disallowed = false;
    for (decl, class) in zip(contracts, classes) {
        match class.validate_version_compatible(list_selector.clone()) {
            Ok(()) => {}

            Err(AllowedLibfuncsError::UnsupportedLibfunc {
                invalid_libfunc,
                allowed_libfuncs_list_name,
            }) => {
                found_disallowed = true;

                let contract_name = decl.submodule_id.name(db);
                let mut diagnostic = formatdoc! {r#"
                    libfunc `{invalid_libfunc}` is not allowed in the libfuncs list `{allowed_libfuncs_list_name}`
                     --> contract: {contract_name}
                "#};

                // If user did not explicitly specify the allowlist, show a help message
                // instructing how to do this. Otherwise, we know that user knows what they
                // do, so we do not clutter compiler output.
                if list_selector == Default::default() {
                    let experimental = BUILTIN_EXPERIMENTAL_LIBFUNCS_LIST;

                    let scarb_toml = unit
                        .main_component()
                        .package
                        .manifest_path()
                        .workspace_relative(ws);

                    let _ = writedoc!(
                        &mut diagnostic,
                        r#"
                            help: try compiling with the `{experimental}` list
                             --> {scarb_toml}
                                [[target.starknet-contract]]
                                allowed-libfuncs-list.name = "{experimental}"
                        "#
                    );
                }

                if props.allowed_libfuncs_deny {
                    ws.config().ui().error(diagnostic);
                } else {
                    ws.config().ui().warn(diagnostic);
                }
            }

            Err(e) => {
                return Err(e).with_context(|| {
                    format!(
                        "failed to check allowed libfuncs for contract: {contract_name}",
                        contract_name = decl.submodule_id.name(db)
                    )
                });
            }
        }
    }

    if found_disallowed && props.allowed_libfuncs_deny {
        bail!("aborting compilation, because contracts use disallowed Sierra libfuncs");
    }

    Ok(())
}
