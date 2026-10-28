use std::sync::{Arc, Mutex};

use anyhow::Result;
use scarb_proc_macro_server_types::methods::Method;

use crate::core::Config;
use crate::ops::store::ProcMacroStore;

pub mod defined_macros;
pub mod expand_attribute;
pub mod expand_derive;
pub mod expand_inline;

pub trait Handler: Method {
    fn handle(
        config: &Config,
        proc_macros: Arc<Mutex<ProcMacroStore>>,
        params: Self::Params,
    ) -> Result<Self::Response>;
}
