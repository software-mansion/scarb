use anyhow::Result;

pub use compilation_unit::*;
pub use profile::*;
pub use repository::*;

use crate::core::Workspace;

mod compilation_unit;
mod compilers;
pub mod helpers;
mod profile;
mod repository;

pub trait Compiler: Sync {
    fn target_kind(&self) -> &str;

    fn compile(&self, unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()>;
}
