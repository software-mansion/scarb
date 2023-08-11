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
        unit: CompilationUnit,
        _db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let _target_dir = unit.target_dir(ws.config());

        let source_path = unit.target().source_path.clone();
        let input_path = unit.main_component().package.root();
        let _input_path = input_path.join(source_path);

        Ok(())
    }
}
