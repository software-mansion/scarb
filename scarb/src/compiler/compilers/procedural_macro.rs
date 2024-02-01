use crate::compiler::{CompilationUnit, Compiler};
use crate::core::{TargetKind, Workspace};
use anyhow::{Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use std::path::Path;
use std::process::Command;
use tracing::trace_span;

pub struct ProceduralMacroCompiler;

impl Compiler for ProceduralMacroCompiler {
    fn target_kind(&self) -> TargetKind {
        TargetKind::CAIRO_PLUGIN.clone()
    }

    fn compile(
        &self,
        unit: CompilationUnit,
        _db: &mut RootDatabase,
        _ws: &Workspace<'_>,
    ) -> Result<()> {
        let main_package = unit.components.first().unwrap().package.clone();
        let mut cmd = Self::build_command(main_package.root());
        {
            let _ = trace_span!("compile_proc_macro").enter();
            let status = cmd
                .status()
                .with_context(|| format!("Failed to execute {:?}", cmd))?;
            if !status.success() {
                return Err(anyhow::anyhow!(
                    "Failed to compile procedural macro plugin: {:?}",
                    cmd
                ));
            }
        }
        Ok(())
    }
}

impl ProceduralMacroCompiler {
    fn build_command(cwd: impl AsRef<Path>) -> Command {
        let mut cmd = Command::new("cargo");
        cmd.current_dir(cwd);
        cmd.arg("build");
        cmd.arg("--release");
        cmd
    }
}
