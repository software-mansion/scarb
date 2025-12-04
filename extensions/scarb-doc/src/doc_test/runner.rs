use crate::AdditionalMetadata;
use crate::doc_test::code_blocks::{CodeBlock, CodeBlockId, count_blocks_per_item};
use crate::doc_test::ui::TestResult;
use crate::doc_test::workspace::TestWorkspace;
use anyhow::Result;
use create_output_dir::create_output_dir;
use scarb_fs_utils::{
    EXECUTE_PRINT_OUTPUT_FILENAME, EXECUTE_PROGRAM_OUTPUT_FILENAME, incremental_create_dir_unique,
};
use scarb_metadata::ScarbCommand;
use scarb_ui::Ui;
use scarb_ui::components::{NewLine, Status};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;

pub type ExecutionResults = HashMap<CodeBlockId, ExecutionResult>;

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub status: TestStatus,
    pub print_output: String,
    pub program_output: String,
    pub outcome: ExecutionOutcome,
}

impl ExecutionResult {
    pub fn as_markdown(&self) -> String {
        let mut output = String::new();
        if !self.print_output.is_empty() {
            output.push_str("\nOutput:\n```\n");
            output.push_str(&self.print_output);
            output.push_str("\n```\n");
        }
        if !self.program_output.is_empty() {
            output.push_str("\nResult:\n```\n");
            output.push_str(&self.program_output);
            output.push_str("\n```\n");
        }
        if output.is_empty() && self.outcome == ExecutionOutcome::RunSuccess {
            output.push_str("\n*No output.*\n");
        }
        output
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TestSummary {
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
}

impl TestSummary {
    pub fn is_ok(&self) -> bool {
        self.failed == 0
    }

    pub fn is_fail(&self) -> bool {
        self.failed > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionOutcome {
    BuildSuccess,
    RunSuccess,
    CompileError,
    RuntimeError,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunStrategy {
    Ignore,
    Build,
    Execute,
}

/// A runner for executing ([`CodeBlock`]) examples found in documentation.
/// Uses the target package as a dependency and runs each code block in an isolated temporary workspace.
/// Relies on `scarb build` and `scarb execute` commands to build and run the examples,
/// based on the requested [`RunStrategy`] for the given code block.
///
/// Note: it is expected examples (`code_blocks`) that this runner executes only depend on the target package and standard libraries.
pub struct TestRunner<'a> {
    /// Metadata of the target package whose documentation is being tested.
    metadata: &'a AdditionalMetadata,
    ui: Ui,
}

impl<'a> TestRunner<'a> {
    pub fn new(metadata: &'a AdditionalMetadata, ui: Ui) -> Self {
        Self { metadata, ui }
    }

    pub fn run_all(&self, code_blocks: &[CodeBlock]) -> Result<(TestSummary, ExecutionResults)> {
        let pkg_name = &self.metadata.name;

        let mut results = HashMap::new();
        let mut summary = TestSummary::default();
        let mut failed_names = Vec::new();
        let blocks_per_item = count_blocks_per_item(code_blocks);

        self.ui.print(Status::new(
            "Running",
            &format!("{} doc examples for `{pkg_name}`", code_blocks.len()),
        ));

        let mut idx = 0;
        for block in code_blocks {
            let strategy = block.run_strategy();
            let total_in_item = *blocks_per_item.get(&block.id.item_full_path).unwrap_or(&1);
            let display_name = block.id.display_name(total_in_item);

            match strategy {
                RunStrategy::Ignore => {
                    summary.ignored += 1;
                    self.ui.print(TestResult::ignored(&display_name));
                }
                _ => {
                    idx += 1;
                    match self.run_single(block, strategy, idx) {
                        Ok(res) => match res.status {
                            TestStatus::Passed => {
                                summary.passed += 1;
                                self.ui.print(TestResult::ok(&display_name));
                                results.insert(block.id.clone(), res);
                            }
                            TestStatus::Failed => {
                                summary.failed += 1;
                                self.ui.print(TestResult::failed(&display_name));
                                failed_names.push(display_name);
                            }
                        },
                        Err(e) => {
                            summary.failed += 1;
                            self.ui.print(TestResult::failed(&display_name));
                            failed_names.push(display_name);
                            self.ui.error(format!("Error running example: {:#}", e));
                        }
                    }
                }
            }
        }
        // TODO: add struct with `impl Message` to display this
        if !failed_names.is_empty() {
            self.ui.print("\nfailures:");
            for display_name in &failed_names {
                self.ui.print(format!("    {}", display_name));
            }
        }
        self.ui.print(NewLine::new());
        self.ui.print(summary.clone());

        Ok((summary, results))
    }

    fn run_single(
        &self,
        code_block: &CodeBlock,
        strategy: RunStrategy,
        index: usize,
    ) -> Result<ExecutionResult> {
        let ws = TestWorkspace::new(self.metadata, index, code_block)?;
        let (actual, print_output, program_output) = self.run_single_inner(&ws, strategy)?;
        let expected = code_block.expected_outcome();
        let status = if actual == expected {
            TestStatus::Passed
        } else {
            match (actual, expected) {
            (ExecutionOutcome::RunSuccess, ExecutionOutcome::RuntimeError) => {
                self.ui.error("Test executable succeeded, but it's marked `should_panic`.");
            }
            (ExecutionOutcome::BuildSuccess, ExecutionOutcome::CompileError) => {
                self.ui.error("Test compiled successfully, but it's marked `compile_fail`.");
            }
            _ => { }
        }
            TestStatus::Failed
        };

        Ok(ExecutionResult {
            outcome: actual,
            status,
            print_output,
            program_output,
        })
    }

    fn run_single_inner(
        &self,
        ws: &TestWorkspace,
        strategy: RunStrategy,
    ) -> Result<(ExecutionOutcome, String, String)> {
        if strategy == RunStrategy::Ignore {
            unreachable!("the code block should be filtered out before reaching here");
        }
        let target_dir = ws.root().join("target");
        let build_result = ScarbCommand::new()
            .arg("build")
            .current_dir(ws.root())
            .env("SCARB_TARGET_DIR", target_dir.as_str())
            .env("SCARB_UI_VERBOSITY", self.ui.verbosity().to_string())
            .env("SCARB_MANIFEST_PATH", ws.manifest_path().as_str())
            .env("SCARB_ALL_FEATURES", "true")
            .run();

        if build_result.is_err() {
            return Ok((ExecutionOutcome::CompileError, String::new(), String::new()));
        } else if strategy == RunStrategy::Build {
            return Ok((ExecutionOutcome::BuildSuccess, String::new(), String::new()));
        }

        let output_dir = target_dir.join("execute").join(ws.package_name());
        create_output_dir(output_dir.as_std_path())?;
        let (output_dir, execution_id) = incremental_create_dir_unique(&output_dir, "execution")?;

        let run_result = ScarbCommand::new()
            .arg("execute")
            .arg("--no-build")
            .arg("--save-print-output")
            .arg("--save-program-output")
            .current_dir(ws.root())
            .env("SCARB_EXECUTION_ID", execution_id.to_string())
            .env("SCARB_TARGET_DIR", target_dir.as_str())
            .env("SCARB_UI_VERBOSITY", self.ui.verbosity().to_string())
            .env("SCARB_MANIFEST_PATH", ws.manifest_path().as_str())
            .env("SCARB_ALL_FEATURES", "true")
            .run();

        if run_result.is_err() {
            Ok((ExecutionOutcome::RuntimeError, String::new(), String::new()))
        } else {
            let print_output = fs::read_to_string(output_dir.join(EXECUTE_PRINT_OUTPUT_FILENAME))
                .unwrap_or_default()
                .trim()
                .to_string();
            let program_output =
                fs::read_to_string(output_dir.join(EXECUTE_PROGRAM_OUTPUT_FILENAME))
                    .unwrap_or_default()
                    .trim()
                    .to_string();
            Ok((ExecutionOutcome::RunSuccess, print_output, program_output))
        }
    }
}
