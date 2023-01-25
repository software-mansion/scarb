use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;

use anyhow::Result;
use itertools::Itertools;
use smol_str::SmolStr;

use crate::compiler::CompilationUnit;
use crate::core::{TargetKind, Workspace};

mod lib;

pub trait TargetCompiler {
    fn compile(&self, unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()>;
}

impl<F> TargetCompiler for F
where
    F: Fn(CompilationUnit, &Workspace<'_>) -> Result<()>,
{
    fn compile(&self, unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
        self(unit, ws)
    }
}

pub struct TargetCompilerMap {
    map: HashMap<SmolStr, Box<dyn TargetCompiler>>,
}

impl TargetCompilerMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn load(&mut self, kind: &TargetKind) -> Result<&dyn TargetCompiler> {
        match self.map.entry(kind.name().into()) {
            Entry::Occupied(entry) => Ok(&**entry.into_mut()),
            Entry::Vacant(entry) => {
                let compiler = Self::build(kind)?;
                Ok(&**entry.insert(compiler))
            }
        }
    }

    fn build(kind: &TargetKind) -> Result<Box<dyn TargetCompiler>> {
        match kind {
            TargetKind::Lib(_) => Ok(Box::new(&lib::compile_lib)),
            TargetKind::External(_) => todo!("External targets are not implemented yet."),
        }
    }
}

impl fmt::Debug for TargetCompilerMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TargetCompilerMap")?;
        f.debug_set().entries(self.map.keys().sorted()).finish()
    }
}
