use crate::compiler::compilers::{Props, SerdeListSelector};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes};
use crate::core::{Utf8PathWorkspaceExt, Workspace};
use anyhow::{bail, ensure, Context};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_defs::ids::NamedLanguageElementId;
use cairo_lang_filesystem::db::{AsFilesGroupMut, FilesGroup};
use cairo_lang_filesystem::flag::Flag;
use cairo_lang_filesystem::ids::FlagId;
use cairo_lang_starknet::contract::ContractDeclaration;
use cairo_lang_starknet_classes::allowed_libfuncs::{
    AllowedLibfuncsError, ListSelector, BUILTIN_EXPERIMENTAL_LIBFUNCS_LIST,
};
use cairo_lang_starknet_classes::contract_class::ContractClass;
use cairo_lang_utils::Upcast;
use indoc::{formatdoc, writedoc};
use std::fmt::Write;
use std::iter::zip;
use tracing::debug;

const AUTO_WITHDRAW_GAS_FLAG: &str = "add_withdraw_gas";

pub fn ensure_gas_enabled(db: &mut RootDatabase) -> anyhow::Result<()> {
    let flag = FlagId::new(db.as_files_group_mut(), AUTO_WITHDRAW_GAS_FLAG);
    let flag = db.get_flag(flag);
    ensure!(
        flag.map(|f| matches!(*f, Flag::AddWithdrawGas(true)))
            .unwrap_or(false),
        "the target starknet contract compilation requires gas to be enabled"
    );
    Ok(())
}

pub fn check_allowed_libfuncs(
    props: &Props,
    contracts: &[ContractDeclaration],
    classes: &[ContractClass],
    db: &RootDatabase,
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

                let contract_name = decl.submodule_id.name(db.upcast());
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
                        contract_name = decl.submodule_id.name(db.upcast())
                    )
                })
            }
        }
    }

    if found_disallowed && props.allowed_libfuncs_deny {
        bail!("aborting compilation, because contracts use disallowed Sierra libfuncs");
    }

    Ok(())
}
