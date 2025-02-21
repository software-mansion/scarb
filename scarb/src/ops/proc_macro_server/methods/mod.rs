use std::sync::Arc;

use anyhow::Result;
use scarb_proc_macro_server_types::methods::Method;

use crate::compiler::plugin::collection::WorkspaceProcMacros;

pub mod defined_macros;
pub mod expand_attribute;
pub mod expand_derive;
pub mod expand_inline;

pub trait Handler: Method {
    fn handle(
        proc_macro_host: Arc<WorkspaceProcMacros>,
        params: Self::Params,
    ) -> Result<Self::Response>;
}
