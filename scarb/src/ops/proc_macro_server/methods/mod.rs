use std::sync::Arc;

use anyhow::Result;
use scarb_proc_macro_server_types::methods::Method;

use crate::compiler::plugin::proc_macro::ProcMacroHost;

pub mod defined_macros;
pub mod expand_attribute;
pub mod expand_derive;

pub trait Handler: Method {
    fn handle(proc_macro_host: Arc<ProcMacroHost>, params: Self::Params) -> Result<Self::Response>;
}
