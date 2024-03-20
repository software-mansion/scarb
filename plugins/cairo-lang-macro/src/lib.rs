pub use cairo_lang_macro_attributes::*;
#[doc(hidden)]
pub use linkme;

use cairo_lang_macro_stable::ffi::StableSlice;
use cairo_lang_macro_stable::{StableAuxData, StableProcMacroResult};
use std::slice;

mod types;

pub use types::*;

/// Free the memory allocated for the [`StableProcMacroResult`].
///
/// This function needs to be accessible through the FFI interface,
/// of the dynamic library re-exporting it.
/// The name of this function will not be mangled by the Rust compiler (through the `no_mangle` attribute).
/// This means that the name will not be extended with neither additional prefixes nor suffixes
/// by the Rust compiler and the corresponding symbol will be available by the name of the function as id.
///
/// # Safety
#[no_mangle]
#[doc(hidden)]
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
#[no_mangle]
#[doc(hidden)]
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
