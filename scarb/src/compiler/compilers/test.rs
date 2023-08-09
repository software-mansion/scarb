use anyhow::Result;
use cairo_lang_compiler::db::RootDatabase;

use crate::compiler::{CompilationUnit, Compiler};
use crate::core::{Target, Workspace};

pub struct TestCompiler;

impl Compiler for TestCompiler {
    fn target_kind(&self) -> &str {
        Target::TEST
    }

    fn compile(
        &self,
        _unit: CompilationUnit,
        _db: &mut RootDatabase,
        _ws: &Workspace<'_>,
    ) -> Result<()> {
        Ok(())
    }
}
