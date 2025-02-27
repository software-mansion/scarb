use std::sync::Arc;

use anyhow::Result;
use scarb_proc_macro_server_types::methods::Method;

use crate::compiler::plugin::collection::WorkspaceProcMacros;

pub mod defined_macros;

#[cfg(not(feature = "macro_v2"))]
pub mod expand_attribute;
#[cfg(feature = "macro_v2")]
#[path = "expand_attribute_v2.rs"]
pub mod expand_attribute;

#[cfg(not(feature = "macro_v2"))]
pub mod expand_derive;
#[cfg(feature = "macro_v2")]
#[path = "expand_derive_v2.rs"]
pub mod expand_derive;

#[cfg(not(feature = "macro_v2"))]
pub mod expand_inline;
#[cfg(feature = "macro_v2")]
#[path = "expand_inline_v2.rs"]
pub mod expand_inline;

pub trait Handler: Method {
    fn handle(
        proc_macro_host: Arc<WorkspaceProcMacros>,
        params: Self::Params,
    ) -> Result<Self::Response>;
}
