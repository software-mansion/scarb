use crate::oracle::connection::Connection;
use anyhow::Result;
use cairo_vm::Felt252;
use url::Url;

pub struct StdioJsonRpcConnection {}

impl StdioJsonRpcConnection {
    pub fn connect(_connection_url: Url) -> Result<Self> {
        Ok(Self {}) // TODO
    }
}

impl Connection for StdioJsonRpcConnection {
    fn call(&mut self, _selector: &str, _calldata: &[Felt252]) -> Result<Vec<Felt252>> {
        Ok(vec![Felt252::from(9876543210u64)]) // TODO
    }
}
