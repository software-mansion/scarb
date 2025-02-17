#![allow(dead_code)]

// These modules are copied from cairo-lang-starknet-classes without any changes.
mod compiler_version;
mod felt252_serde;
mod felt252_vec_compression;
mod keccak;

#[allow(unused)]
#[path = "../../../scarb/src/internal/fsx.rs"]
mod fsx;

use crate::felt252_serde::sierra_from_felt252s;

use cairo_lang_starknet_classes::allowed_libfuncs::{
    AllowedLibfuncs, AllowedLibfuncsError, ListSelector,
};
use cairo_lang_starknet_classes::contract_class::ContractClass;

/// Checks that all the used libfuncs in the contract class are allowed in the contract class
/// sierra version.
pub fn validate_version_compatible(
    class: &ContractClass,
    list_selector: ListSelector,
) -> Result<(), AllowedLibfuncsError> {
    let list_name = list_selector.to_string();
    let allowed_libfuncs = lookup_allowed_libfuncs_list(list_selector)?;
    let (_, _, sierra_program) = sierra_from_felt252s(&class.sierra_program)
        .map_err(|_| AllowedLibfuncsError::SierraProgramError)?;
    for libfunc in sierra_program.libfunc_declarations.iter() {
        if !allowed_libfuncs
            .allowed_libfuncs
            .contains(&libfunc.long_id.generic_id)
        {
            return Err(AllowedLibfuncsError::UnsupportedLibfunc {
                invalid_libfunc: libfunc.long_id.generic_id.to_string(),
                allowed_libfuncs_list_name: list_name,
            });
        }
    }
    Ok(())
}

/// The allowed libfuncs list to use if no list is supplied to the compiler.
/// Should only contain libfuncs that are audited and tested.
const BUILTIN_AUDITED_LIBFUNCS_LIST: &str = "audited";
/// The allowed libfuncs list to use allowed on testnet2 - should be all libfuncs currently
/// supported by starknet.
const BUILTIN_EXPERIMENTAL_LIBFUNCS_LIST: &str = "experimental";
/// The experimental list contains all the libfuncs and is currently used for development.
const BUILTIN_ALL_LIBFUNCS_LIST: &str = "all";

/// Returns the sierra version corresponding to the given version id.
fn lookup_allowed_libfuncs_list(
    list_selector: ListSelector,
) -> Result<AllowedLibfuncs, AllowedLibfuncsError> {
    let list_name = list_selector.to_string();
    let allowed_libfuncs_str: String = match list_selector {
        ListSelector::ListName(list_name) => match list_name.as_str() {
            BUILTIN_ALL_LIBFUNCS_LIST => {
                include_str!("allowed_libfuncs_lists/all.json").to_string()
            }
            BUILTIN_EXPERIMENTAL_LIBFUNCS_LIST => {
                include_str!("allowed_libfuncs_lists/experimental.json").to_string()
            }
            BUILTIN_AUDITED_LIBFUNCS_LIST => {
                include_str!("allowed_libfuncs_lists/audited.json").to_string()
            }
            _ => {
                return Err(AllowedLibfuncsError::UnexpectedAllowedLibfuncsList {
                    allowed_libfuncs_list_name: list_name.to_string(),
                });
            }
        },
        ListSelector::ListFile(file_path) => fsx::read_to_string(&file_path).map_err(|_| {
            AllowedLibfuncsError::UnknownAllowedLibfuncsFile {
                allowed_libfuncs_list_file: file_path,
            }
        })?,
        ListSelector::DefaultList => {
            include_str!("allowed_libfuncs_lists/audited.json").to_string()
        }
    };
    let allowed_libfuncs: Result<AllowedLibfuncs, serde_json::Error> =
        serde_json::from_str(&allowed_libfuncs_str);
    allowed_libfuncs.map_err(|_| AllowedLibfuncsError::DeserializationError {
        allowed_libfuncs_list_file: list_name,
    })
}
