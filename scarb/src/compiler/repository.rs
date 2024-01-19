use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;

use anyhow::{bail, Result};
use cairo_lang_compiler::db::RootDatabase;
use itertools::Itertools;
use smol_str::SmolStr;

use crate::compiler::compilers::{LibCompiler, StarknetContractCompiler, TestCompiler};
use crate::compiler::helpers::build_compiler_config;
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
        repo.add(Box::new(TestCompiler)).unwrap();
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

    pub fn compile(
        &self,
        unit: CompilationUnit,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let target_kind = &unit.target().kind;
        let Some(compiler) = self.compilers.get(target_kind.as_str()) else {
            bail!("unknown compiler for target `{target_kind}`");
        };
        compiler.compile(unit, db, ws)
    }

    pub fn check(
        &self,
        unit: CompilationUnit,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let mut compiler_config = build_compiler_config(&unit, ws);
        compiler_config.diagnostics_reporter.ensure(db)?;
        Ok(())
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
