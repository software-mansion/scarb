use crate::oracle::connections::stdio_jsonrpc::StdioJsonRpcConnection;
use anyhow::{Error, Result, anyhow, bail};
use cairo_vm::Felt252;
use std::collections::HashMap;
use std::rc::Rc;
use url::Url;

pub trait Connection {
    fn call(&mut self, selector: &str, calldata: &[Felt252]) -> Result<Vec<Felt252>>;
}

/// Maintains a collection of oracle [`Connection`]s.
pub struct ConnectionManager(HashMap<String, Result<Box<dyn Connection + 'static>, Rc<Error>>>);

impl ConnectionManager {
    pub fn new() -> Self {
        Self(Default::default())
    }

    /// Establishes a connection to a given connection string and stores it in the connection pool.
    ///
    /// If the connection already exists in the pool, the existing connection is reused.
    /// The same applies to connection errors, the pool will never reattempt to reconnect.
    pub fn connect(&mut self, connection_string: &str) -> Result<&mut (dyn Connection + 'static)> {
        self.0
            .entry(connection_string.into())
            .or_insert_with(|| Self::create_connection(connection_string).map_err(Rc::new))
            .as_mut()
            .map(AsMut::as_mut)
            .map_err(|e| {
                // We're OK with flattening the error object here because it is going to be
                // stringified when encoding the response.
                anyhow!("{e}")
            })
    }

    fn create_connection(connection_string: &str) -> Result<Box<dyn Connection + 'static>> {
        let connection_url = Url::parse(connection_string)?;
        match connection_url.scheme() {
            "stdio" => Ok(Box::new(StdioJsonRpcConnection::connect(connection_url)?)),
            _ => bail!(
                "unsupported connection scheme: {connection_string}\n\
                note: supported schemes are: `stdio`"
            ),
        }
    }
}
