use anyhow::Result;

pub use compilation_unit::*;
pub use profile::*;

use crate::core::Workspace;

mod compilation_unit;
mod profile;
pub(crate) mod targets;

pub trait Compiler {
    fn compile(&self, unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()>;
}

impl<F> Compiler for F
where
    F: Fn(CompilationUnit, &Workspace<'_>) -> Result<()>,
{
    fn compile(&self, unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
        self(unit, ws)
    }
}
