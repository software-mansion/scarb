use anyhow::Result;

pub use compilation_unit::*;
pub use profile::*;

use crate::core::Workspace;

mod compilation_unit;
pub mod helpers;
mod profile;
pub(crate) mod targets;

pub trait Compiler: Sync {
    fn target_kind(&self) -> &str;

    fn compile(&self, unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()>;
}
