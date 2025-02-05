use crate::compiler::helpers::write_json;
use crate::compiler::helpers::{build_compiler_config, collect_main_crate_ids};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, Compiler};
use crate::core::{TargetKind, Utf8PathWorkspaceExt, Workspace};
use anyhow::{bail, ensure, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_executable::compile::{
    compile_executable_function_in_prepared_db, CompiledFunction, ExecutableConfig,
};
use cairo_lang_executable::executable::Executable;
use cairo_lang_executable::plugin::{EXECUTABLE_PREFIX, EXECUTABLE_RAW_ATTR};
use cairo_lang_filesystem::ids::CrateId;
use cairo_lang_lowering::ids::ConcreteFunctionWithBodyId;
use cairo_lang_sierra_generator::executables::find_executable_function_ids;
use cairo_lang_utils::Upcast;
use camino::Utf8Path;
use indoc::formatdoc;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tracing::trace_span;

pub struct ExecutableCompiler;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Props {
    pub allow_syscalls: bool,
    pub function: Option<String>,
}

impl Compiler for ExecutableCompiler {
    fn target_kind(&self) -> TargetKind {
        TargetKind::EXECUTABLE.clone()
    }

    fn compile(
        &self,
        unit: CairoCompilationUnit,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        ensure!(
            !unit.compiler_config.enable_gas,
            formatdoc! {r#"
                executable target cannot be compiled with enabled gas calculation
                help: if you want to diable gas calculation, consider adding following
                excerpt to your package manifest
                    -> {scarb_toml}
                        [cairo]
                        enable-gas = false
                "#, scarb_toml=unit.main_component().package.manifest_path().workspace_relative(ws)}
        );

        let props: Props = unit.main_component().target_props()?;

        let target_dir = unit.target_dir(ws);
        let main_crate_ids = collect_main_crate_ids(&unit, db);
        let compiler_config = build_compiler_config(db, &unit, &main_crate_ids, ws);
        let span = trace_span!("compile_executable");
        let executable = {
            let _guard = span.enter();
            Executable::new(compile_executable(
                &unit,
                db,
                ws,
                props.function.as_deref(),
                main_crate_ids,
                compiler_config.diagnostics_reporter,
                ExecutableConfig {
                    allow_syscalls: props.allow_syscalls,
                },
            )?)
        };

        write_json(
            format!("{}.executable.json", unit.main_component().target_name()).as_str(),
            "output file",
            &target_dir,
            ws,
            &executable,
        )
    }
}

fn compile_executable(
    unit: &CairoCompilationUnit,
    db: &RootDatabase,
    ws: &Workspace<'_>,
    executable_path: Option<&str>,
    main_crate_ids: Vec<CrateId>,
    mut diagnostics_reporter: DiagnosticsReporter<'_>,
    config: ExecutableConfig,
) -> Result<CompiledFunction> {
    let executables = find_executable_functions(db, main_crate_ids, executable_path);

    let executable = match executables.len() {
        0 => {
            // Report diagnostics as they might reveal the reason why no executable was found.
            diagnostics_reporter.ensure(db)?;
            bail!("Requested `#[executable]` not found.");
        }
        1 => executables[0],
        _ => {
            let executable_names = executables
                .iter()
                .map(|executable| originating_function_path(db, *executable))
                .sorted()
                .collect_vec();
            let scarb_toml = unit
                .main_component()
                .package
                .manifest_path()
                .workspace_relative(ws);
            bail!(multiple_executables_error_message(
                executable_names,
                scarb_toml
            ));
        }
    };

    compile_executable_function_in_prepared_db(db, executable, diagnostics_reporter, config)
}

fn multiple_executables_error_message(executables: Vec<String>, scarb_toml: &Utf8Path) -> String {
    let executable_names = executables.clone().join("\n\t");

    let manifest = executables
        .iter()
        .map(|function| {
            let name = function
                .clone()
                .split("::")
                .last()
                .map(ToString::to_string)
                .unwrap_or_else(|| function.clone());
            formatdoc! {r#"
                [[target.executable]]
                name = "{name}"
                function = "{function}"
            "#,
            }
        })
        .join("\n");

    formatdoc! {r#"
        more than one executable found in the main crate: 
            {}
        help: add a separate `executable` target for each of your executable functions
        -> {scarb_toml}
        {manifest}
        "#,
        executable_names
    }
}

/// Search crates identified by `main_crate_ids` for functions annotated with `#[executable]` attribute.
/// If `executable_path` is provided, only functions with exactly the same path will be returned.
fn find_executable_functions(
    db: &RootDatabase,
    main_crate_ids: Vec<CrateId>,
    executable_path: Option<&str>,
) -> Vec<ConcreteFunctionWithBodyId> {
    let mut executables: Vec<_> = find_executable_function_ids(db, main_crate_ids)
        .into_iter()
        .filter_map(|(id, labels)| {
            labels
                .into_iter()
                .any(|label| label == EXECUTABLE_RAW_ATTR)
                .then_some(id)
        })
        .collect();

    if let Some(executable_path) = executable_path {
        executables
            .retain(|executable| originating_function_path(db, *executable) == executable_path);
    };
    executables
}

/// Returns the path to the function that the executable is wrapping.
///
/// If the executable is not wrapping a function, returns the full path of the executable.
fn originating_function_path(db: &RootDatabase, wrapper: ConcreteFunctionWithBodyId) -> String {
    let semantic = wrapper.base_semantic_function(db);
    let wrapper_name = semantic.name(db);
    let wrapper_full_path = semantic.full_path(db.upcast());
    let Some(function_name) = wrapper_name.strip_prefix(EXECUTABLE_PREFIX) else {
        return wrapper_full_path;
    };
    let Some(wrapper_path_to_module) = wrapper_full_path.strip_suffix(wrapper_name.as_str()) else {
        return wrapper_full_path;
    };
    format!("{}{}", wrapper_path_to_module, function_name)
}
