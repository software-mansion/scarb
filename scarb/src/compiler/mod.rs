use crate::compiler::incremental::IncrementalContext;
use crate::core::{TargetKind, Workspace};
use crate::internal::offloader::Offloader;
use anyhow::Result;
use cairo_lang_compiler::db::RootDatabase;
pub use compilation_unit::*;
pub use profile::*;
pub use repository::*;

mod compilation_unit;
mod compilers;
pub mod db;
pub mod helpers;
pub mod incremental;
pub mod plugin;
mod profile;
mod repository;
mod syntax;

pub trait Compiler: Sync {
    fn target_kind(&self) -> TargetKind;

    fn compile(
        &self,
        unit: &CairoCompilationUnit,
        ctx: &IncrementalContext,
        offloader: &Offloader<'_>,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()>;
}
