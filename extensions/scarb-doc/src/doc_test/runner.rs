use crate::AdditionalMetadata;
use crate::doc_test::code_blocks::{CodeBlock, CodeBlockId, count_blocks_per_item};
use crate::doc_test::ui::TestResult;
use crate::doc_test::workspace::DocTestWorkspace;
use anyhow::{Context, Result};
use fs_extra::dir::{CopyOptions, copy};
use itertools::Itertools;
use rayon::prelude::*;
use scarb_metadata::ScarbCommand;
use scarb_ui::Ui;
use scarb_ui::components::{NewLine, Status};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::TempDir;
use tempfile::tempdir;

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
    has_lib_target: bool,
    ui: Ui,
    /// Target directory shared between all doc test runs.
    /// This allows speeding up doc tests compilation by sharing incremental caches.
    target_dir: TempDir,
    /// Build profile passed to `scarb build` / `scarb execute` and used to locate the
    /// incremental cache directory (`target/<profile>/`).
    profile: String,
}

impl<'a> TestRunner<'a> {
    pub fn new(metadata: &'a AdditionalMetadata, has_lib_target: bool, ui: Ui) -> Result<Self> {
        let target_dir =
            tempdir().context("failed to create directory for doc tests target directory")?;
        let profile = std::env::var("SCARB_PROFILE").unwrap_or_else(|_| "dev".to_string());
        Ok(Self {
            metadata,
            has_lib_target,
            ui,
            target_dir,
            profile,
        })
    }

    pub fn run_all(&self, code_blocks: &[CodeBlock]) -> Result<(TestSummary, ExecutionResults)> {
        let pkg_name = &self.metadata.name;

        let mut results = HashMap::new();
        let mut summary = TestSummary::default();
        let mut failed_names = Vec::new();
        let blocks_per_item = count_blocks_per_item(code_blocks);
        type IndexedBlock<'b> = (usize, &'b CodeBlock);
        let indexed_blocks = code_blocks
            .iter()
            .enumerate()
            .collect::<Vec<IndexedBlock<'_>>>();

        self.ui.print(Status::new(
            "Running",
            &format!("{} doc examples for `{pkg_name}`", code_blocks.len()),
        ));

        let execution_blocks: Vec<(usize, &CodeBlock)> = indexed_blocks
            .iter()
            .filter(|(_, block)| block.run_strategy() != RunStrategy::Ignore)
            .map(|(order, block)| (*order, *block))
            .collect();

        let mut run_results = HashMap::new();

        // Run non-compile_fail blocks sequentially until the first successful compilation to warm
        // the incremental cache. Compile_fail blocks are skipped here and always run in parallel.
        for (order, block) in &execution_blocks {
            if block.expected_outcome() == ExecutionOutcome::CompileError {
                continue;
            }
            let result = self.run_single(
                block,
                block.run_strategy(),
                order + 1,
                self.target_dir.path(),
            );
            let build_succeeded = matches!(
                &result,
                Ok(r) if r.outcome != ExecutionOutcome::CompileError
            );
            run_results.insert(*order, result);
            if build_succeeded {
                break;
            }
        }

        let parallel_results = execution_blocks
            .iter()
            .filter(|(order, _)| !run_results.contains_key(order))
            .collect_vec()
            .into_par_iter()
            .map(|(order, block)| {
                let target_dir = tempdir()
                    .context("failed to create temporary target directory for doc test run")?;
                let display_name = Self::display_name(block, &blocks_per_item);
                copy_incremental_cache(self.target_dir.path(), target_dir.path(), &self.profile)
                    .context(format!(
                        "failed to copy incremental cache for `{}`",
                        display_name
                    ))?;
                let result =
                    self.run_single(block, block.run_strategy(), *order + 1, target_dir.path());
                Ok::<(usize, Result<ExecutionResult>), anyhow::Error>((*order, result))
            })
            .collect::<Vec<_>>();

        for result in parallel_results {
            let (order, run_result) = result?;
            run_results.insert(order, run_result);
        }

        for (order, block) in indexed_blocks {
            match block.run_strategy() {
                RunStrategy::Ignore => {
                    summary.ignored += 1;
                    let display_name = Self::display_name(block, &blocks_per_item);
                    self.ui.print(TestResult::ignored(&display_name));
                }
                _ => match run_results.remove(&order) {
                    Some(Ok(res)) => match res.status {
                        TestStatus::Passed => {
                            summary.passed += 1;
                            let display_name = Self::display_name(block, &blocks_per_item);
                            self.ui.print(TestResult::ok(&display_name));
                            results.insert(block.id.clone(), res);
                        }
                        TestStatus::Failed => {
                            summary.failed += 1;
                            let display_name = Self::display_name(block, &blocks_per_item);
                            self.report_mismatch(res.outcome, block.expected_outcome());
                            self.ui.print(res.print_output.clone());
                            self.ui.print(res.program_output.clone());
                            self.ui.print(TestResult::failed(&display_name));
                            failed_names.push(display_name);
                        }
                    },
                    Some(Err(e)) => {
                        summary.failed += 1;
                        let display_name = Self::display_name(block, &blocks_per_item);
                        self.ui.print(TestResult::failed(&display_name));
                        failed_names.push(display_name);
                        self.ui.error(format!("Error running example: {:#}", e));
                    }
                    None => {
                        summary.failed += 1;
                        let display_name = Self::display_name(block, &blocks_per_item);
                        self.ui.print(TestResult::failed(&display_name));
                        failed_names.push(display_name);
                        self.ui.error("Missing doc test run result.");
                    }
                },
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

    fn report_mismatch(&self, actual: ExecutionOutcome, expected: ExecutionOutcome) {
        match (actual, expected) {
            (ExecutionOutcome::RunSuccess, ExecutionOutcome::RuntimeError) => {
                self.ui
                    .error("Test executable succeeded, but it's marked `should_panic`.");
            }
            (ExecutionOutcome::BuildSuccess, ExecutionOutcome::CompileError) => {
                self.ui
                    .error("Test compiled successfully, but it's marked `compile_fail`.");
            }
            _ => {}
        }
    }

    fn display_name(block: &CodeBlock, blocks_per_item: &HashMap<String, usize>) -> String {
        let total_in_item = *blocks_per_item.get(&block.id.item_full_path).unwrap_or(&1);
        block.id.display_name(total_in_item)
    }

    fn run_single(
        &self,
        code_block: &CodeBlock,
        strategy: RunStrategy,
        index: usize,
        target_dir: &Path,
    ) -> Result<ExecutionResult> {
        let ws = DocTestWorkspace::new(
            self.metadata,
            index,
            code_block,
            self.has_lib_target,
            &self.ui,
        )?;
        let (actual, print_output, program_output) =
            self.run_single_inner(&ws, strategy, target_dir)?;
        let expected = code_block.expected_outcome();
        let status = if actual == expected {
            TestStatus::Passed
        } else {
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
        ws: &DocTestWorkspace,
        strategy: RunStrategy,
        target_dir: &Path,
    ) -> Result<(ExecutionOutcome, String, String)> {
        if strategy == RunStrategy::Ignore {
            unreachable!("the code block should be filtered out before reaching here");
        }
        let build_result = ScarbCommand::new()
            .arg("build")
            .current_dir(ws.root())
            .env("SCARB_TARGET_DIR", target_dir)
            .env("SCARB_UI_VERBOSITY", self.ui.verbosity().to_string())
            .env("SCARB_MANIFEST_PATH", ws.manifest_path().as_str())
            .env("SCARB_ALL_FEATURES", "true")
            .env("SCARB_PROFILE", &self.profile)
            .run();

        if build_result.is_err() {
            return Ok((ExecutionOutcome::CompileError, String::new(), String::new()));
        } else if strategy == RunStrategy::Build {
            return Ok((ExecutionOutcome::BuildSuccess, String::new(), String::new()));
        }

        let scarb_path = std::env::var("SCARB").unwrap_or_else(|_| "scarb".to_string());
        let run_result = Command::new(scarb_path)
            .args([
                "--json",
                "execute",
                "--no-build",
                "--output=none",
                "--print-program-output",
            ])
            .current_dir(ws.root())
            .env("SCARB_TARGET_DIR", target_dir)
            .env("SCARB_UI_VERBOSITY", "no-warnings")
            .env("SCARB_MANIFEST_PATH", ws.manifest_path().as_str())
            .env("SCARB_ALL_FEATURES", "true")
            .env("SCARB_PROFILE", &self.profile)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output();

        match run_result {
            Err(_) => Ok((ExecutionOutcome::RuntimeError, String::new(), String::new())),
            Ok(output) if !output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let print_output = parse_print_output(&stdout);
                let error_message = parse_last_error_message(&stdout);

                Ok((ExecutionOutcome::RuntimeError, print_output, error_message))
            }
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let print_output = parse_print_output(&stdout);
                let program_output = parse_program_output(&stdout);
                Ok((ExecutionOutcome::RunSuccess, print_output, program_output))
            }
        }
    }
}

/// Copies only the `.fingerprints` and `incremental` subdirectories from
/// `from/<profile>/` into `to/<profile>/`.
/// This is enough to seed the incremental compilation cache for parallel doc test runs without
/// copying large compiled artifacts.
fn copy_incremental_cache(from: &Path, to: &Path, profile: &str) -> Result<()> {
    let profile_from = from.join(profile);
    let profile_to = to.join(profile);
    for cache_dir in [".fingerprint", "incremental"] {
        let src = profile_from.join(cache_dir);
        if !src.exists() {
            continue;
        }
        let dst = profile_to.join(cache_dir);
        fs::create_dir_all(&dst)
            .with_context(|| format!("failed to create `{}`", dst.display()))?;
        copy(
            &src,
            &dst,
            &CopyOptions {
                content_only: true,
                ..Default::default()
            },
        )?;
    }
    Ok(())
}

#[derive(Deserialize)]
struct ExecutionOutputLine {
    program_output: String,
}

#[derive(Deserialize)]
struct ExecutionErrorLine {
    #[serde(rename = "type")]
    message_type: String,
    message: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct StatusMessage {
    status: String,
    message: String,
}

fn parse_print_output(stdout: &str) -> String {
    let mut lines: Vec<&str> = stdout.lines().collect();
    // Drop the last line if it's an execution error or program output.
    // This should be obtained with parsing functions defined below.
    if lines
        .last()
        .map(|line| {
            serde_json::from_str::<ExecutionOutputLine>(line).is_ok()
                || serde_json::from_str::<ExecutionErrorLine>(line)
                    .ok()
                    .map(|parsed| parsed.message_type == "error")
                    .unwrap_or_default()
        })
        .unwrap_or_default()
    {
        let _ = lines.pop();
    }
    lines
        .into_iter()
        .filter(|line| {
            // Filter out status message marking execution start.
            let Some(parsed) = serde_json::from_str::<StatusMessage>(line).ok() else {
                return true;
            };
            parsed.status != "executing"
        })
        .collect_vec()
        .join("\n")
        .trim()
        .to_string()
}

fn parse_program_output(stdout: &str) -> String {
    stdout
        .lines()
        .next_back()
        .and_then(|last| serde_json::from_str::<ExecutionOutputLine>(last).ok())
        .map(|parsed| parsed.program_output)
        .unwrap_or_default()
}

fn parse_last_error_message(stdout: &str) -> String {
    stdout
        .lines()
        .next_back()
        .and_then(|last| serde_json::from_str::<ExecutionErrorLine>(last).ok())
        .filter(|parsed| parsed.message_type == "error")
        .map(|parsed| parsed.message)
        .unwrap_or_default()
}
