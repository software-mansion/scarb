use crate::code_blocks::{CodeBlock, CodeBlockId};
use crate::types::crate_type::Crate;
use crate::types::module_type::Module;
use crate::types::other_types::ItemData;
use anyhow::{Context, Result, anyhow};
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
use std::fs;
use tempfile::tempdir;

pub type ExecutionResults = HashMap<CodeBlockId, ExecutionResult>;

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub outcome: ExecutionOutcome,
    pub print_output: String,
    pub program_output: String,
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
        if output.is_empty() {
            output.push_str("\n*No output.*\n");
        }
        output
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionOutcome {
    Success,
    CompileError,
    RuntimeError,
    None,
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
pub enum RunStrategy {
    Ignore,
    Build,
    Run,
}

/// A runner for executing examples (code blocks) found in documentation.
/// Uses `scarb execute` and runs code blocks in isolated temporary workspaces.
pub struct DocTestRunner<'a> {
    package_metadata: &'a PackageMetadata,
    ui: Ui,
}

impl<'a> DocTestRunner<'a> {
    pub fn new(package_metadata: &'a PackageMetadata, ui: Ui) -> Self {
        Self {
            package_metadata,
            ui,
        }
    }

    pub fn execute(&self, code_blocks: &[CodeBlock]) -> Result<ExecutionResults> {
        let mut results = HashMap::new();
        for (index, code_block) in code_blocks.iter().enumerate() {
            let result = self.execute_single(code_block, index)?;
            results.insert(code_block.id.clone(), result);
        }
        Ok(results)
    }

    fn execute_single(&self, code_block: &CodeBlock, index: usize) -> Result<ExecutionResult> {
        let temp_dir =
            tempdir().context("failed to create temporary workspace for doc snippet execution")?;
        let project_dir = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf())
            .map_err(|path| anyhow!("path `{}` is not UTF-8 encoded", path.display()))?;

        self.write_manifest(&project_dir, index)?;
        self.write_lib_cairo(&project_dir, code_block)?;

        let (print_output, program_output) = self.run_execute(&project_dir, index, code_block)?;

        Ok(ExecutionResult {
            print_output,
            program_output,
            outcome: ExecutionOutcome::Success,
        })
    }

    // TODO: consider using `ProjectBuilder` instead.
    // - multiple snippets per package
    // - or multiple packages with snippets in a workspace with common dep
    fn write_manifest(&self, dir: &Utf8Path, index: usize) -> Result<()> {
        let package_name = &self.package_metadata.name;
        let package_dir = self
            .package_metadata
            .manifest_path
            .parent()
            .context("package manifest path has no parent directory")?;

        let name = self.generated_package_name(index);
        let dependency_path = format!("\"{}\"", package_dir);
        let manifest = formatdoc! {r#"
            [package]
            name = "{name}"
            version = "0.1.0"
            edition = "2024_07"

            [dependencies]
            {package_name} = {{ path = {dependency_path} }}
            cairo_execute = "{CAIRO_VERSION}"

            [cairo]
            enable-gas = false

            [executable]
        "#};
        fs::write(dir.join("Scarb.toml"), manifest)
            .context("failed to write manifest for example")?;
        Ok(())
    }

    fn write_lib_cairo(&self, dir: &Utf8Path, code_block: &CodeBlock) -> Result<()> {
        let package_name = &self.package_metadata.name;
        let src_dir = dir.join("src");
        fs::create_dir_all(&src_dir).context("failed to create src directory")?;

        let mut body = String::new();
        for line in code_block.content.lines() {
            if line.trim().is_empty() {
                body.push_str("    \n");
            } else {
                body.push_str("    ");
                body.push_str(line);
                body.push('\n');
            }
        }

        let lib_cairo = formatdoc! {r#"
            use {package_name}::*;

            #[executable]
            fn main() {{
            {body}
            }}
        "# };
        fs::write(src_dir.join("lib.cairo"), lib_cairo).context("failed to write lib.cairo")?;
        Ok(())
    }

    fn run_execute(
        &self,
        project_dir: &Utf8Path,
        index: usize,
        code_block: &CodeBlock,
    ) -> Result<(String, String)> {
        let target_dir = project_dir.join("target");
        let output_dir = target_dir
            .join("execute")
            .join(self.generated_package_name(index));
        create_output_dir(output_dir.as_std_path())?;
        let (output_dir, execution_id) = incremental_create_execution_output_dir(&output_dir)?;

        self.ui.print(Status::new(
            "Executing",
            format!("example #{} from `{}`", index, code_block.id.item_full_path).as_str(),
        ));
        ScarbCommand::new()
            .arg("execute")
            // .args(["--executable-function", "main"])
            .arg("--save-program-output")
            .arg("--save-print-output")
            .current_dir(project_dir)
            .env("SCARB_EXECUTION_ID", execution_id.to_string())
            .env("SCARB_TARGET_DIR", target_dir.as_str())
            .env("SCARB_UI_VERBOSITY", self.ui.verbosity().to_string())
            .env(
                "SCARB_MANIFEST_PATH",
                project_dir.join("Scarb.toml").as_str(),
            )
            .env("SCARB_ALL_FEATURES", "true")
            .run()
            .with_context(|| "execution failed")?;

        let print_output_file = output_dir.join(EXECUTE_PRINT_OUTPUT_FILENAME);
        let print_output = fs::read_to_string(&print_output_file).with_context(|| {
            format!(
                "failed to read execution print output from file: {}",
                print_output_file
            )
        })?;
        let program_output_file = output_dir.join(EXECUTE_PROGRAM_OUTPUT_FILENAME);
        let program_output = fs::read_to_string(&program_output_file).with_context(|| {
            format!(
                "failed to read program output from file: {}",
                program_output_file
            )
        })?;
        Ok((
            print_output.trim().to_string(),
            program_output.trim().to_string(),
        ))
    }

    fn generated_package_name(&self, index: usize) -> String {
        let package_name = &self.package_metadata.name;
        format!("{package_name}_example_{index}")
    }
}

/// Collects all runnable `DocCodeBlock`s from the crate.
pub fn collect_runnable_code_blocks(crate_: &Crate<'_>) -> Vec<CodeBlock> {
    let mut runnable_code_blocks = Vec::new();
    collect_from_module(&crate_.root_module, &mut runnable_code_blocks);
    // TODO: should these be ignored?
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
        if block.run_strategy() == RunStrategy::Run {
            runnable_code_blocks.push(block.clone());
        }
    }
}
