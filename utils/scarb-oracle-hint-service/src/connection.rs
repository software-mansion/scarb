use crate::protocol::Protocols;
use anyhow::Result;
use starknet_core::types::Felt;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;

pub trait Connection {
    fn call(&mut self, selector: &str, calldata: &[Felt]) -> Result<Vec<Felt>>;
}

/// Maintains a collection of oracle [`Connection`]s.
#[derive(Default)]
pub struct Connections(HashMap<String, Box<dyn Connection + 'static>>);

impl Connections {
    /// Establishes a connection to a given connection string and stores it in the connection pool.
    ///
    /// If the connection already exists in the pool, the existing connection is reused.
    /// Erroneous connections aren't cached, and further connections will attempt to reconnect.
    pub fn connect(
        &mut self,
        connection_string: &str,
        protocols: &Protocols,
    ) -> Result<&mut (dyn Connection + 'static)> {
        match self.0.entry(connection_string.into()) {
            Entry::Occupied(entry) => Ok(entry.into_mut().as_mut()),
            Entry::Vacant(entry) => {
                Ok(entry.insert(protocols.connect(connection_string)?).as_mut())
            }
        }
    }
}

impl fmt::Debug for Connections {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut connection_strings = self.0.keys().collect::<Vec<_>>();
        connection_strings.sort();
        f.debug_tuple("Connections")
            .field(&connection_strings)
            .finish()
    }
}
