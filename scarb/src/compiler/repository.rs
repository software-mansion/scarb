use anyhow::{Result, bail};
use cairo_lang_compiler::db::RootDatabase;
use itertools::Itertools;
use smol_str::SmolStr;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;

use crate::compiler::compilers::{
    ExecutableCompiler, LibCompiler, StarknetContractCompiler, TestCompiler,
};
use crate::compiler::incremental::{load_incremental_artifacts, save_incremental_artifacts};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, Compiler};
use crate::core::Workspace;
use crate::internal::offloader::Offloader;

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
        repo.add(Box::new(ExecutableCompiler)).unwrap();
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
        unit: CairoCompilationUnit,
        offloader: &Offloader<'_>,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let target_kind = &unit.main_component().target_kind();
        let Some(compiler) = self.compilers.get(target_kind.as_str()) else {
            bail!("unknown compiler for target `{target_kind}`");
        };
        let ctx = load_incremental_artifacts(&unit, db, ws)?;
        compiler.compile(&unit, &ctx, offloader, db, ws)?;
        save_incremental_artifacts(&unit, db, ctx, ws)?;
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
