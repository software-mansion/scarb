use crate::Connection;
use anyhow::{Result, bail};
use std::fmt;

pub trait Protocol {
    const SCHEME: &'static str;
    fn connect(connection_string: &str) -> Result<Box<dyn Connection + 'static>>;
}

type ConnectFunc = fn(&str) -> Result<Box<dyn Connection + 'static>>;

// This uses a vector because it is not expected to contain >16 protocols (16 is the thumb rule as
// vector into hashmap switch point).
#[derive(Default)]
pub struct Protocols(Vec<(&'static str, ConnectFunc)>);

impl Protocols {
    pub fn add<P: Protocol>(&mut self) {
        for (scheme, connect) in &mut self.0 {
            if *scheme == P::SCHEME {
                *connect = P::connect;
                return;
            }
        }
        self.0.push((P::SCHEME, P::connect));
    }

    pub fn connect(&self, connection_string: &str) -> Result<Box<dyn Connection + 'static>> {
        for (scheme, connect) in &self.0 {
            if let Some(command) = connection_string.strip_prefix(scheme)
                && let Some(command) = command.strip_prefix(':')
            {
                return connect(command);
            }
        }

        bail!(
            "unsupported connection scheme: {connection_string:?}\n\
            note: supported schemes are: {schemes}",
            schemes = self
                .0
                .iter()
                .map(|(scheme, _)| format!("{scheme:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl fmt::Debug for Protocols {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let schemes = self.0.iter().map(|(scheme, _)| *scheme).collect::<Vec<_>>();
        f.debug_tuple("ProtocolRepository").field(&schemes).finish()
    }
}
