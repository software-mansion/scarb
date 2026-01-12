use crate::compiler::compilers::{
    ExecutableCompiler, LibCompiler, StarknetContractCompiler, TestCompiler,
};
use crate::compiler::incremental::{IncrementalContext, warmup_incremental_cache};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, Compiler};
use crate::core::Workspace;
use crate::internal::offloader::Offloader;
use anyhow::{Result, bail};
use cairo_lang_utils::CloneableDatabase;
use itertools::Itertools;
use smol_str::SmolStr;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;
use std::sync::Arc;

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
        unit: &CairoCompilationUnit,
        ctx: Arc<IncrementalContext>,
        offloader: &Offloader<'_>,
        db: &mut dyn CloneableDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let target_kind = &unit.main_component().target_kind();
        let Some(compiler) = self.compilers.get(target_kind.as_str()) else {
            bail!("unknown compiler for target `{target_kind}`");
        };
        let cached_crates = ctx.cached_crates().to_vec();
        // We run incremental cache warmup in parallel with the compilation.
        // This operation is "fire and forget".
        // We do not want to block waiting for warmup to finish, as the compiler itself will
        // block if needed.
        let warmup_db = db.dyn_clone();
        rayon::spawn(move || warmup_incremental_cache(warmup_db.as_ref(), cached_crates));
        compiler.compile(unit, ctx, offloader, db, ws)?;
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
