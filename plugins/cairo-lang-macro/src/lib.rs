//! <br>
//!
//! **A library for writing Cairo procedural macros in Rust.**
//! <br>
//!
//! # Cairo procedural macro
//!
//! A Cairo procedural macro is a dynamic library that can be loaded by
//! [Scarb](https://github.com/software-mansion/scarb) package manager during the project build.
//! The goal of procedural macros is to provide dynamic code generation capabilities to Cairo
//! programmers.
//!
//! The code generation should be implemented as a Rust function, that takes [`TokenStream`] as
//! input and returns [`ProcMacroResult`] as output.
//! The function implementing the macro should be wrapped with [`attribute_macro`].
//!

pub use cairo_lang_macro_attributes::*;
#[doc(hidden)]
pub use linkme;

use cairo_lang_macro_stable::ffi::StableSlice;
use cairo_lang_macro_stable::{StableAuxData, StableExpansionsList, StableProcMacroResult};
use std::ffi::{c_char, CStr};
use std::slice;

mod types;

pub use types::*;

#[doc(hidden)]
#[derive(Clone)]
pub struct ExpansionDefinition {
    pub name: &'static str,
    pub kind: ExpansionKind,
    pub fun: ExpansionFunc,
}

type ExpansionFunc = fn(TokenStream) -> ProcMacroResult;

/// Distributed slice for storing procedural macro code expansion capabilities.
///
/// Each element denotes name of the macro, and the expand function pointer.
#[doc(hidden)]
#[linkme::distributed_slice]
pub static MACRO_DEFINITIONS_SLICE: [ExpansionDefinition];

/// This function discovers expansion capabilities defined by the procedural macro.
///
/// This function needs to be accessible through the FFI interface,
/// of the dynamic library re-exporting it.
///
/// # Safety
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn list_expansions() -> StableExpansionsList {
    let list = MACRO_DEFINITIONS_SLICE
        .iter()
        .map(|m| m.clone().into_stable())
        .collect();
    StableSlice::new(list)
}

/// Free the memory allocated for the [`StableProcMacroResult`].
///
/// This function needs to be accessible through the FFI interface,
/// of the dynamic library re-exporting it.
///
/// # Safety
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn free_expansions_list(list: StableExpansionsList) {
    let v = list.into_owned();
    v.into_iter().for_each(|v| {
        ExpansionDefinition::free_owned(v);
    });
}

/// The code expansion callback.
///
/// This function needs to be accessible through the FFI interface,
/// of the dynamic library re-exporting it.
///
/// The function will be called for each code expansion by the procedural macro.
///
/// # Safety
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn expand(
    item_name: *const c_char,
    stable_token_stream: cairo_lang_macro_stable::StableTokenStream,
) -> cairo_lang_macro_stable::StableResultWrapper {
    let token_stream = TokenStream::from_stable(&stable_token_stream);
    let item_name = CStr::from_ptr(item_name).to_string_lossy().to_string();
    let fun = MACRO_DEFINITIONS_SLICE
        .iter()
        .find_map(|m| {
            if m.name == item_name.as_str() {
                Some(m.fun)
            } else {
                None
            }
        })
        .expect("procedural macro not found");
    let result = fun(token_stream);
    let result: StableProcMacroResult = result.into_stable();
    cairo_lang_macro_stable::StableResultWrapper {
        input: stable_token_stream,
        output: result,
    }
}

/// Free the memory allocated for the [`StableProcMacroResult`].
///
/// This function needs to be accessible through the FFI interface,
/// of the dynamic library re-exporting it.
/// The name of this function will not be mangled by the Rust compiler (through the `no_mangle` attribute).
/// This means that the name will not be extended with neither additional prefixes nor suffixes
/// by the Rust compiler and the corresponding symbol will be available by the name of the function as id.
///
/// # Safety
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn free_result(result: StableProcMacroResult) {
    ProcMacroResult::from_owned_stable(result);
}

/// Distributed slice for storing auxiliary data collection callback pointers.
#[doc(hidden)]
#[linkme::distributed_slice]
pub static AUX_DATA_CALLBACKS: [fn(Vec<AuxData>)];

/// The auxiliary data collection callback.
///
/// This function needs to be accessible through the FFI interface,
/// of the dynamic library re-exporting it.
///
/// The function will be called for each procedural macro, regardless if it redefines the callback
/// behaviour or not. In case no custom behaviour is defined, this is a no-op.
///
/// # Safety
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn aux_data_callback(
    stable_aux_data: StableSlice<StableAuxData>,
) -> StableSlice<StableAuxData> {
    if !AUX_DATA_CALLBACKS.is_empty() {
        // Callback has been defined, applying the aux data collection.
        let (ptr, n) = stable_aux_data.raw_parts();
        let aux_data: &[StableAuxData] = slice::from_raw_parts(ptr, n);
        let aux_data = aux_data
            .iter()
            .filter_map(|a| AuxData::from_stable(a))
            .collect::<Vec<_>>();
        for fun in AUX_DATA_CALLBACKS {
            fun(aux_data.clone());
        }
    }
    stable_aux_data
}
