use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;

use anyhow::Result;
use itertools::Itertools;
use smol_str::SmolStr;

pub use lib::*;
pub use starknet_contract::*;

use crate::compiler::Compiler;
use crate::core::TargetKind;

mod lib;
mod starknet_contract;

pub struct TargetCompilerMap {
    map: HashMap<SmolStr, Box<dyn Compiler>>,
}

impl TargetCompilerMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn load(&mut self, kind: &TargetKind) -> Result<&dyn Compiler> {
        match self.map.entry(kind.name().into()) {
            Entry::Occupied(entry) => Ok(&**entry.into_mut()),
            Entry::Vacant(entry) => {
                let compiler = Self::build(kind)?;
                Ok(&**entry.insert(compiler))
            }
        }
    }

    fn build(kind: &TargetKind) -> Result<Box<dyn Compiler>> {
        match kind {
            TargetKind::Lib(_) => Ok(Box::new(LibCompiler)),
            TargetKind::External(ext) => {
                // TODO(mkaput): starknet-contract should be implemented as an extension.
                if ext.kind_name == "starknet-contract" {
                    return Ok(Box::new(StarknetContractCompiler));
                }

                todo!("External targets are not implemented yet.")
            }
        }
    }
}

impl fmt::Debug for TargetCompilerMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TargetCompilerMap")?;
        f.debug_set().entries(self.map.keys().sorted()).finish()
    }
}
