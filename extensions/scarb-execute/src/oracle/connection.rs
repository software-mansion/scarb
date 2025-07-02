use anyhow::{Error, Result, anyhow};
use cairo_vm::Felt252;
use std::collections::HashMap;
use std::rc::Rc;

/// Maintains a collection of oracle [`Connection`]s.
pub struct ConnectionManager(HashMap<String, Result<Connection, Rc<Error>>>);

impl ConnectionManager {
    pub fn new() -> Self {
        Self(Default::default())
    }

    /// Establishes a connection to a given connection string and stores it in the connection pool.
    ///
    /// If the connection already exists in the pool, the existing connection is reused.
    /// The same applies to connection errors, the pool will never reattempt to reconnect.
    pub fn connect(&mut self, connection_string: &str) -> Result<&mut Connection> {
        self.0
            .entry(connection_string.into())
            .or_insert_with(|| Connection::connect(connection_string).map_err(Rc::new))
            .as_mut()
            .map_err(|e| {
                // We're OK with flattening the error object here because it is going to be
                // stringified when encoding the response.
                anyhow!("{e}")
            })
    }
}

pub struct Connection {}

impl Connection {
    fn connect(_connection_string: &str) -> Result<Self> {
        // TODO
        Ok(Self {})
    }

    pub fn call(&mut self, _selector: &str, _calldata: &[Felt252]) -> Result<Vec<Felt252>> {
        // TODO
        Ok(vec![Felt252::from(9876543210u64)])
    }
}
