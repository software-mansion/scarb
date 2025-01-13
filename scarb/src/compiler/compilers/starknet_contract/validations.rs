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
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_starknet_classes::contract_class::ContractClass;
use cairo_lang_utils::Upcast;
use indoc::{formatdoc, writedoc};
use std::fmt::Write;
use std::iter::zip;
use tracing::debug;

const AUTO_WITHDRAW_GAS_FLAG: &str = "add_withdraw_gas";
const MAX_SIERRA_PROGRAM_FELTS: usize = 81290;
const MAX_CASM_PROGRAM_FELTS: usize = 81290;
const MAX_CONTRACT_CLASS_BYTES: usize = 4089446;
const MAX_COMPILED_CONTRACT_CLASS_BYTES: usize = 4089446;

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

pub fn check_sierra_size_limits(classes: &[ContractClass], ws: &Workspace<'_>) {
    for class in classes {
        let sierra_felts = class.sierra_program.len();
        if sierra_felts > MAX_SIERRA_PROGRAM_FELTS {
            ws.config().ui().warn(formatdoc! {r#"
                Sierra program exceeds maximum byte-code size on Starknet:
                {MAX_SIERRA_PROGRAM_FELTS} felts allowed. Actual size: {sierra_felts} felts.
            "#});
        }

        let class_size = serde_json::to_vec(class).unwrap().len();
        if class_size > MAX_CONTRACT_CLASS_BYTES {
            ws.config().ui().warn(formatdoc! {r#"
                Contract class size exceeds maximum allowed size on Starknet:
                {MAX_CONTRACT_CLASS_BYTES} bytes allowed. Actual size: {class_size} bytes.
            "#});
        }
    }
}

pub fn check_casm_size_limits(casm_classes: &[Option<CasmContractClass>], ws: &Workspace<'_>) {
    for casm_class in casm_classes.iter().flatten() {
        let casm_felts = casm_class.bytecode.len();
        if casm_felts > MAX_CASM_PROGRAM_FELTS {
            ws.config().ui().warn(formatdoc! {r#"
                CASM program exceeds maximum byte-code size on Starknet:
                {MAX_CASM_PROGRAM_FELTS} felts allowed. Actual size: {casm_felts} felts.
            "#});
        }

        let compiled_class_size = serde_json::to_vec(casm_class).unwrap().len();
        if compiled_class_size > MAX_COMPILED_CONTRACT_CLASS_BYTES {
            ws.config().ui().warn(formatdoc! {r#"
                Compiled contract class size exceeds maximum allowed size on Starknet:
                {MAX_COMPILED_CONTRACT_CLASS_BYTES} bytes allowed. Actual size: {compiled_class_size} bytes.
            "#});
        }
    }
}
