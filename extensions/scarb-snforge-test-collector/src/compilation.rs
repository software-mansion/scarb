use anyhow::Result;
use cairo_lang_sierra::program::{ProgramArtifact, VersionedProgram};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;
use snforge_test_collector_interface::{CrateLocation, TestCaseRaw};

use crate::compilation::test_collector::collect_tests;
use crate::crate_collection::TestCompilationTarget;
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
) -> Result<Vec<CompiledTestCrateRaw>> {
    targets
        .par_iter()
        .map(|target| target.compile_tests(compilation_unit))
        .collect()
}

impl TestCompilationTarget {
    fn compile_tests(&self, compilation_unit: &CompilationUnit) -> Result<CompiledTestCrateRaw> {
        let (sierra_program, test_cases) = collect_tests(
            &self.crate_name,
            self.crate_root.as_std_path(),
            &self.lib_content,
            compilation_unit,
        )?;

        Ok(CompiledTestCrateRaw {
            sierra_program: VersionedProgram::v1(ProgramArtifact::stripped(sierra_program)),
            test_cases,
            tests_location: self.crate_location.clone(),
        })
    }
}
