use anyhow::Result;
use cairo_lang_compiler::db::RootDatabase;
use std::io::Write;

use crate::compiler::{CompilationUnit, Compiler};
use crate::core::{Target, Workspace};
use scarb_test_collector::collect_tests;

pub struct TestCompiler;

impl Compiler for TestCompiler {
    fn target_kind(&self) -> &str {
        Target::TEST
    }

    fn compile(
        &self,
        unit: CompilationUnit,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let target_dir = unit.target_dir(ws.config());
        let package_name = unit.main_component().package.id.name.as_str();

        let source_path = unit.target().source_path.clone();
        let input_path = unit.main_component().package.root();
        let input_path = input_path.join(source_path);

        let (sierra_program, test_cases) = collect_tests(db, input_path.as_str(), package_name)?;

        {
            let mut file = target_dir.open_rw(
                format!("test.{}.sierra", unit.target().name),
                "output file",
                ws.config(),
            )?;
            file.write_all(sierra_program.to_string().as_bytes())?;
        }

        {
            let mut file = target_dir.open_rw(
                format!("test.{}.json", unit.target().name),
                "output file",
                ws.config(),
            )?;
            file.write_all(serde_json::to_string(&test_cases)?.as_bytes())?;
        }

        Ok(())
    }
}
