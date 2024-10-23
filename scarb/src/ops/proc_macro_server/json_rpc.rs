use crate::compiler::plugin::proc_macro::ProcMacroHost;
use anyhow::Result;
use proc_macro_server_api::Method;
use serde::Serialize;
use std::sync::Arc;

pub trait Handler: Method {
    fn handle(proc_macros: Arc<ProcMacroHost>, params: Self::Params) -> Result<Self::Response>;
}

#[derive(Serialize)]
pub struct ErrResponse {
    message: String,
}

impl ErrResponse {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}
