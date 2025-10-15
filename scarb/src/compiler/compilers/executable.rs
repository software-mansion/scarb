use crate::compiler::db::{has_plugin, is_executable_plugin};
use crate::compiler::helpers::write_json;
use crate::compiler::helpers::{build_compiler_config, collect_main_crate_ids};
use crate::compiler::incremental::IncrementalContext;
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, Compiler};
use crate::core::{PackageName, TargetKind, Utf8PathWorkspaceExt, Workspace};
use crate::internal::offloader::Offloader;
use anyhow::{Result, bail, ensure};
use cairo_lang_compiler::{
    CompilerConfig, compile_prepared_db_program_artifact_for_functions, ensure_diagnostics,
};
use cairo_lang_executable::compile::{
    CompiledFunction, ExecutableConfig, compile_executable_function_in_prepared_db,
};
use cairo_lang_executable::executable::Executable;
use cairo_lang_executable_plugin::{EXECUTABLE_PREFIX, EXECUTABLE_RAW_ATTR};
use cairo_lang_filesystem::ids::CrateId;
use cairo_lang_lowering::ids::ConcreteFunctionWithBodyId;
use cairo_lang_sierra::program::VersionedProgram;
use cairo_lang_sierra_generator::executables::find_executable_function_ids;
use camino::Utf8Path;
use indoc::formatdoc;
use itertools::Itertools;
use salsa::Database;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::trace_span;

pub struct ExecutableCompiler;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Props {
    pub allow_syscalls: bool,
    pub function: Option<String>,
    pub sierra: bool,
}

impl Compiler for ExecutableCompiler {
    fn target_kind(&self) -> TargetKind {
        TargetKind::EXECUTABLE.clone()
    }

    fn compile(
        &self,
        unit: &CairoCompilationUnit,
        ctx: &IncrementalContext,
        offloader: &Offloader<'_>,
        db: &dyn Database,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        ensure!(
            !unit.compiler_config.enable_gas,
            formatdoc! {r#"
                executable target cannot be compiled with enabled gas calculation
                help: if you want to disable gas calculation, consider adding following
                excerpt to your package manifest
                    -> {scarb_toml}
                        [cairo]
                        enable-gas = false
                "#, scarb_toml=ws.manifest_path().workspace_relative(ws)
            }
        );

        check_executable_plugin_dependency(unit, ws, db, &unit.main_component().package.id.name);

        let props: Props = unit.main_component().targets.target_props()?;

        let main_crate_ids = collect_main_crate_ids(unit, db);
        let compiler_config = build_compiler_config(db, unit, &main_crate_ids, ctx, ws);
        let span = trace_span!("compile_executable");
        let executable = {
            let _guard = span.enter();
            Executable::new(compile_executable(
                unit,
                db,
                ws,
                offloader,
                &props,
                main_crate_ids,
                compiler_config,
                ExecutableConfig {
                    allow_syscalls: props.allow_syscalls,
                    ..ExecutableConfig::default()
                },
            )?)
        };

        let span = trace_span!("serialize_executable_json");
        {
            let _guard = span.enter();
            let target_name = unit.main_component().target_name();
            let target_dir = unit.target_dir(ws);
            offloader.offload("output file", move |ws| {
                write_json(
                    format!("{target_name}.executable.json").as_str(),
                    "output file",
                    &target_dir,
                    ws,
                    &executable,
                )
            });
        }

        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn compile_executable<'db>(
    unit: &CairoCompilationUnit,
    db: &'db dyn Database,
    ws: &Workspace<'_>,
    offloader: &Offloader<'_>,
    props: &Props,
    main_crate_ids: Vec<CrateId<'db>>,
    mut compiler_config: CompilerConfig<'db>,
    config: ExecutableConfig,
) -> Result<CompiledFunction> {
    ensure_diagnostics(db, &mut compiler_config.diagnostics_reporter)?;

    let executable_path = props.function.as_deref();
    let executables = find_executable_functions(db, main_crate_ids, executable_path);

    let executable = match executables.len() {
        0 => {
            bail!("requested `#[executable]` not found");
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

    if props.sierra {
        let span = trace_span!("compile_sierra");
        let program_artifact = {
            let _guard = span.enter();

            // This seems wasteful, as the subsequent call to `compile_executable_function_in_prepared_db`
            // will compile executable to sierra again. However, it's very inexpensive in practice,
            // as salsa will save this compilation in the cache, and only load later.
            let program_artifact = compile_prepared_db_program_artifact_for_functions(
                db,
                vec![executable],
                compiler_config,
            )?;
            Arc::new(program_artifact)
        };

        let span = trace_span!("offload_serializing_sierra");
        {
            let _guard = span.enter();
            let target_name = unit.main_component().target_name();
            let target_dir = unit.target_dir(ws);
            // We only clone Arc, not the underlying program, so it's inexpensive.
            let program = program_artifact.clone();
            offloader.offload("output file", move |ws| {
                // Cloning the underlying program is expensive, but we can afford it here,
                // as we are on a dedicated thread anyway.
                let sierra_program: VersionedProgram = program.as_ref().clone().into();
                write_json(
                    &format!("{target_name}.executable.sierra.json"),
                    "output file",
                    &target_dir,
                    ws,
                    &sierra_program,
                )?;
                Ok(())
            });
        }
    }

    let span = trace_span!("compile_executable_function_in_prepared_db");
    Ok({
        let _guard = span.enter();
        compile_executable_function_in_prepared_db(db, executable, config)?.compiled_function
    })
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
        help: specify a separate `executable` target for each of your executable functions
        -> {scarb_toml}
        {manifest}
        "#,
        executable_names
    }
}

/// Search crates identified by `main_crate_ids` for functions annotated with `#[executable]` attribute.
/// If `executable_path` is provided, only functions with exactly the same path will be returned.
fn find_executable_functions<'db>(
    db: &'db dyn Database,
    main_crate_ids: Vec<CrateId<'db>>,
    executable_path: Option<&str>,
) -> Vec<ConcreteFunctionWithBodyId<'db>> {
    let mut executables: Vec<_> = find_executable_function_ids(db, main_crate_ids)
        .into_iter()
        .filter_map(|(id, labels)| {
            labels
                .into_iter()
                .any(|label| label.to_string(db) == EXECUTABLE_RAW_ATTR)
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
fn originating_function_path<'db>(
    db: &'db dyn Database,
    wrapper: ConcreteFunctionWithBodyId<'db>,
) -> String {
    let semantic = wrapper.base_semantic_function(db);
    let wrapper_name = semantic.name(db).to_string(db);
    let wrapper_full_path = semantic.full_path(db);
    let Some(function_name) = wrapper_name.strip_prefix(EXECUTABLE_PREFIX) else {
        return wrapper_full_path;
    };
    let Some(wrapper_path_to_module) = wrapper_full_path.strip_suffix(&wrapper_name) else {
        return wrapper_full_path;
    };
    format!("{wrapper_path_to_module}{function_name}")
}

fn check_executable_plugin_dependency(
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
    db: &dyn Database,
    package_name: &PackageName,
) {
    let main_component = unit.main_component();

    if main_component.target_kind() == TargetKind::EXECUTABLE
        && !has_plugin(db, is_executable_plugin, main_component)
    {
        ws.config().ui().warn(formatdoc! {
            r#"
            package `{package_name}` declares `executable` target, but does not depend on `cairo_execute` package
            note: this may cause contract compilation to fail with cryptic errors
            help: add dependency on `cairo_execute` to package manifest
             --> {scarb_toml}
                [dependencies]
                cairo_execute = "{cairo_version}"
            "#,
            scarb_toml=unit.main_component().package.manifest_path().workspace_relative(ws),
            cairo_version = crate::version::get().cairo.version,
        })
    }
}
