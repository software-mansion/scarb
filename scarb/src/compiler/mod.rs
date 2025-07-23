use anyhow::Result;
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_filesystem::ids::CrateId;
pub use compilation_unit::*;
pub use profile::*;
pub use repository::*;

use crate::core::{TargetKind, Workspace};
use crate::internal::artifacts_writer::ArtifactsWriterRequestSink;

mod compilation_unit;
mod compilers;
pub mod db;
pub mod helpers;
pub mod incremental;
pub mod plugin;
mod profile;
mod repository;

pub trait Compiler: Sync {
    fn target_kind(&self) -> TargetKind;

    fn compile(
        &self,
        unit: &CairoCompilationUnit,
        cached_crates: &[CrateId],
        artifacts_writer: ArtifactsWriterRequestSink,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()>;
}
