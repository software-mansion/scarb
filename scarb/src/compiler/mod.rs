use crate::compiler::incremental::IncrementalContext;
use crate::core::{TargetKind, Workspace};
use crate::internal::offloader::Offloader;
use anyhow::Result;
use cairo_lang_utils::CloneableDatabase;
pub use compilation_unit::*;
pub use profile::*;
pub use repository::*;
use std::sync::Arc;

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
        ctx: Arc<IncrementalContext>,
        offloader: &Offloader<'_>,
        db: &dyn CloneableDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()>;
}
