use anyhow::Result;
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_filesystem::ids::CrateId;
pub use compilation_unit::*;
pub use profile::*;
pub use repository::*;
use std::sync::mpsc;

use crate::core::{TargetKind, Workspace};
use crate::internal::artifacts_writer::Request;

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
        artifacts_writer: mpsc::Sender<Request>,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()>;
}
