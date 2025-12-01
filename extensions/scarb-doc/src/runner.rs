use crate::code_blocks::{CodeBlock, CodeBlockId};
use crate::types::crate_type::Crate;
use crate::types::module_type::Module;
use crate::types::other_types::ItemData;
use anyhow::{Context, Result, anyhow};
use cairo_lang_filesystem::db::Edition;
use camino::{Utf8Path, Utf8PathBuf};
use create_output_dir::create_output_dir;
use indoc::formatdoc;
use scarb_build_metadata::CAIRO_VERSION;
use scarb_execute_utils::{
    EXECUTE_PRINT_OUTPUT_FILENAME, EXECUTE_PROGRAM_OUTPUT_FILENAME,
    incremental_create_execution_output_dir,
};
use scarb_metadata::{PackageMetadata, ScarbCommand};
use scarb_ui::Ui;
use scarb_ui::components::Status;
use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use tempfile::{TempDir, tempdir};

pub type ExecutionResults = HashMap<CodeBlockId, ExecutionResult>;

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub status: TestStatus,
    pub print_output: String,
    pub program_output: String,
    pub outcome: ExecutionOutcome,
}

impl ExecutionResult {
    /// Formats the execution result as markdown with code blocks.
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

#[derive(Debug, Clone, Default)]
pub struct ExecutionSummary {
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStrategy {
    Ignore,
    Build,
    Execute,
}

/// A runner for executing examples (code blocks) found in documentation.
/// Uses `scarb execute` and runs code blocks in isolated temporary workspaces.
pub struct TestRunner<'a> {
    package_metadata: &'a PackageMetadata,
    ui: Ui,
}

impl<'a> TestRunner<'a> {
    pub fn new(package_metadata: &'a PackageMetadata, ui: Ui) -> Self {
        Self {
            package_metadata,
            ui,
        }
    }

    pub fn execute(
        &self,
        code_blocks: &[CodeBlock],
    ) -> Result<(ExecutionSummary, ExecutionResults)> {
        let mut results = HashMap::new();
        let mut summary = ExecutionSummary::default();
        let (to_run, ignored): (Vec<_>, Vec<_>) = code_blocks
            .iter()
            .partition(|block| block.run_strategy() != RunStrategy::Ignore);

        self.ui.print(Status::new(
            "Found",
            &format!("{} doc tests; {} ignored", code_blocks.len(), ignored.len(),),
        ));
        self.ui.print(Status::new(
            "Running",
            &format!("{} doc tests", to_run.len()),
        ));
        for (index, code_block) in to_run.iter().enumerate() {
            let strategy = code_block.run_strategy();
            match self.execute_single(code_block, index, strategy) {
                Ok(res) => {
                    if res.status == TestStatus::Passed {
                        summary.passed += 1;
                        results.insert(code_block.id.clone(), res);
                    } else {
                        summary.failed += 1;
                    }
                }
                Err(e) => {
                    summary.failed += 1;
                    self.ui
                        .error(format!("Error running example #{}: {:#}", index, e));
                }
            }
        }

        Ok((summary, results))
    }

    fn execute_single(
        &self,
        code_block: &CodeBlock,
        index: usize,
        strategy: RunStrategy,
    ) -> Result<ExecutionResult> {
        let ws = TestWorkspace::new(self.package_metadata, index, code_block)?;
        self.ui.print(Status::new(
            "Running",
            format!("example #{} from `{}`", ws.index, ws.item_full_path).as_str(),
        ));
        let (actual, print_output, program_output) = self.run_test(&ws, strategy)?;
        let expected = code_block.expected_outcome();
        let status = if actual == expected {
            TestStatus::Passed
        } else {
            TestStatus::Failed
        };
        match status {
            TestStatus::Passed => self.ui.print(Status::new(
                "Passed",
                &format!("example #{} from `{}`", ws.index, ws.item_full_path),
            )),
            TestStatus::Failed => self.ui.print(Status::new(
                "Failed",
                &format!("example #{} from `{}`", ws.index, ws.item_full_path),
            )),
        }

        Ok(ExecutionResult {
            status,
            print_output,
            program_output,
            outcome: actual,
        })
    }

    fn run_test(
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
        }
        if strategy == RunStrategy::Build {
            return Ok((ExecutionOutcome::RunSuccess, String::new(), String::new()));
        }
        let output_dir = target_dir.join("execute").join(&ws.package_name);
        create_output_dir(output_dir.as_std_path())?;
        let (output_dir, execution_id) = incremental_create_execution_output_dir(&output_dir)?;

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
            return Ok((ExecutionOutcome::RuntimeError, String::new(), String::new()));
        }
        let print_output = fs::read_to_string(output_dir.join(EXECUTE_PRINT_OUTPUT_FILENAME))
            .unwrap_or_default()
            .trim()
            .to_string();
        let program_output = fs::read_to_string(output_dir.join(EXECUTE_PROGRAM_OUTPUT_FILENAME))
            .unwrap_or_default()
            .trim()
            .to_string();
        Ok((ExecutionOutcome::RunSuccess, print_output, program_output))
    }
}

struct TestWorkspace {
    _temp_dir: TempDir,
    root: Utf8PathBuf,
    manifest_path: Utf8PathBuf,
    package_name: String,
    index: usize,
    item_full_path: String,
}

impl TestWorkspace {
    fn new(metadata: &PackageMetadata, index: usize, code_block: &CodeBlock) -> Result<Self> {
        let temp_dir = tempdir().context("failed to create temporary workspace")?;
        let root = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf())
            .map_err(|path| anyhow!("path `{}` is not UTF-8 encoded", path.display()))?;

        let package_name = format!("{}_example_{}", metadata.name, index);
        let manifest_path = root.join("Scarb.toml");

        let workspace = Self {
            _temp_dir: temp_dir,
            root,
            manifest_path,
            package_name,
            index,
            item_full_path: code_block.id.item_full_path.clone(),
        };
        workspace.write_manifest(metadata)?;
        workspace.write_src(&code_block.content, &metadata.name)?;

        Ok(workspace)
    }

    fn root(&self) -> &Utf8Path {
        &self.root
    }

    fn manifest_path(&self) -> &Utf8Path {
        &self.manifest_path
    }

    fn write_manifest(&self, metadata: &PackageMetadata) -> Result<()> {
        let package_dir = metadata
            .manifest_path
            .parent()
            .context("package manifest path has no parent directory")?;

        let dep = &metadata.name;
        let dep_path = format!("\"{}\"", package_dir);
        let name = &self.package_name;
        let edition = edition_variant(Edition::latest());

        let manifest = formatdoc! {r#"
            [package]
            name = "{name}"
            version = "0.1.0"
            edition = "{edition}"

            [dependencies]
            {dep} = {{ path = {dep_path} }}
            cairo_execute = "{CAIRO_VERSION}"

            [cairo]
            enable-gas = false

            [executable]
        "#
        };
        fs::write(&self.manifest_path, manifest).context("failed to write manifest for example")?;
        Ok(())
    }

    fn write_src(&self, content: &str, package_name: &str) -> Result<()> {
        let src_dir = self.root.join("src");
        fs::create_dir_all(&src_dir).context("failed to create src directory")?;

        let mut body = String::with_capacity(content.len() + content.lines().count() * 5);
        for line in content.lines() {
            writeln!(body, "    {}", line)?;
        }
        let lib_cairo = formatdoc! {r#"
            use {package_name}::*;

            #[executable]
            fn main() {{
            {body}
            }}
        "#};
        fs::write(src_dir.join("lib.cairo"), lib_cairo).context("failed to write lib.cairo")?;
        Ok(())
    }
}

pub fn collect_code_blocks(crate_: &Crate<'_>) -> Vec<CodeBlock> {
    let mut runnable_code_blocks = Vec::new();
    collect_from_module(&crate_.root_module, &mut runnable_code_blocks);
    for module in &crate_.foreign_crates {
        collect_from_module(module, &mut runnable_code_blocks);
    }
    // Sort to run deterministically
    runnable_code_blocks.sort_by_key(|block| block.id.clone());
    runnable_code_blocks
}

fn collect_from_module(module: &Module<'_>, runnable_code_blocks: &mut Vec<CodeBlock>) {
    for item_data in module.get_all_item_ids().values() {
        collect_from_item_data(item_data, runnable_code_blocks);
    }
    for item_data in module.pub_uses.get_all_item_ids().values() {
        collect_from_item_data(item_data, runnable_code_blocks);
    }
}

fn collect_from_item_data(item_data: &ItemData<'_>, runnable_code_blocks: &mut Vec<CodeBlock>) {
    for block in &item_data.code_blocks {
        runnable_code_blocks.push(block.clone());
    }
}

fn edition_variant(edition: Edition) -> String {
    let edition = serde_json::to_value(edition).unwrap();
    let serde_json::Value::String(edition) = edition else {
        panic!("Edition should always be a string.")
    };
    edition
}
