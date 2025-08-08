use crate::connections::stdio_jsonrpc::StdioJsonRpcConnection;
use anyhow::{Result, bail};
use starknet_core::types::Felt;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;

pub trait Connection {
    fn call(&mut self, selector: &str, calldata: &[Felt]) -> Result<Vec<Felt>>;
}

/// Maintains a collection of oracle [`Connection`]s.
#[derive(Default)]
pub struct ConnectionManager(HashMap<String, Box<dyn Connection + 'static>>);

impl ConnectionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Establishes a connection to a given connection string and stores it in the connection pool.
    ///
    /// If the connection already exists in the pool, the existing connection is reused.
    /// Erroneous connections aren't cached, and further connections will attempt to reconnect.
    pub fn connect(&mut self, connection_string: &str) -> Result<&mut (dyn Connection + 'static)> {
        match self.0.entry(connection_string.into()) {
            Entry::Occupied(entry) => Ok(entry.into_mut().as_mut()),
            Entry::Vacant(entry) => Ok(entry
                .insert(Self::create_connection(connection_string)?)
                .as_mut()),
        }
    }

    fn create_connection(connection_string: &str) -> Result<Box<dyn Connection + 'static>> {
        if let Some(command) = connection_string.strip_prefix("stdio:") {
            Ok(Box::new(StdioJsonRpcConnection::connect(command)?))
        } else {
            bail!(
                "unsupported connection scheme: {connection_string:?}\n\
                note: supported schemes are: `stdio`"
            )
        }
    }
}

impl fmt::Debug for ConnectionManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut connection_strings = self.0.keys().collect::<Vec<_>>();
        connection_strings.sort();
        f.debug_tuple("ConnectionManager")
            .field(&connection_strings)
            .finish()
    }
}
