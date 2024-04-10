use anyhow::Result;
use cairo_lang_sierra::program::VersionedProgram;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;

use crate::compilation::test_collector::{collect_tests, TestCaseRaw};
use crate::crate_collection::{CrateLocation, TestCompilationTarget};
use crate::metadata::CompilationUnit;

pub mod test_collector;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CompiledTestCrateRaw {
    pub sierra_program: VersionedProgram,
    pub test_cases: Vec<TestCaseRaw>,
    pub tests_location: CrateLocation,
}

pub fn compile_tests(
    targets: &Vec<TestCompilationTarget>,
    compilation_unit: &CompilationUnit,
    generate_statements_functions_mappings: bool,
) -> Result<Vec<CompiledTestCrateRaw>> {
    targets
        .par_iter()
        .map(|target| {
            target.compile_tests(compilation_unit, generate_statements_functions_mappings)
        })
        .collect()
}

impl TestCompilationTarget {
    fn compile_tests(
        &self,
        compilation_unit: &CompilationUnit,
        generate_statements_functions_mappings: bool,
    ) -> Result<CompiledTestCrateRaw> {
        let (program_artifact, test_cases) = collect_tests(
            &self.crate_name,
            self.crate_root.as_std_path(),
            &self.lib_content,
            compilation_unit,
            generate_statements_functions_mappings,
        )?;

        Ok(CompiledTestCrateRaw {
            sierra_program: program_artifact.into(),
            test_cases,
            tests_location: self.crate_location.clone(),
        })
    }
}
