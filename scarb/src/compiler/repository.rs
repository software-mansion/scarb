use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;

use anyhow::{bail, Result};
use itertools::Itertools;
use smol_str::SmolStr;

use crate::compiler::compilers::{LibCompiler, StarknetContractCompiler};
use crate::compiler::{CompilationUnit, Compiler};
use crate::core::Workspace;

pub struct CompilerRepository {
    compilers: HashMap<SmolStr, Box<dyn Compiler>>,
}

impl CompilerRepository {
    pub fn empty() -> Self {
        Self {
            compilers: HashMap::new(),
        }
    }

    pub fn std() -> Self {
        let mut repo = Self::empty();
        repo.add(Box::new(LibCompiler)).unwrap();
        repo.add(Box::new(StarknetContractCompiler)).unwrap();
        repo
    }

    pub fn add(&mut self, compiler: Box<dyn Compiler>) -> Result<()> {
        let target_kind = compiler.target_kind().into();
        match self.compilers.entry(target_kind) {
            Entry::Occupied(e) => bail!("found duplicate compiler for target `{}`", e.key()),
            Entry::Vacant(e) => {
                e.insert(compiler);
                Ok(())
            }
        }
    }

    pub fn compile(&self, unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
        let target_kind = &unit.target.kind;
        let Some(compiler) = self.compilers.get(target_kind) else {
            bail!("unknown compiler for target `{target_kind}`");
        };
        compiler.compile(unit, ws)
    }
}

impl fmt::Debug for CompilerRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CompilerRepository ")?;
        f.debug_set()
            .entries(self.compilers.keys().sorted())
            .finish()
    }
}
