#![deny(clippy::dbg_macro)]
#![deny(clippy::disallowed_methods)]

use crate::hint_processor::ExecuteHintProcessor;
use crate::output::{
    ExecutionOutput, ExecutionResources, ExecutionResourcesSource, ExecutionSummary,
};
use crate::profiler::{build_profiler_call_trace, get_profiler_tracked_resource};
use anyhow::{Context, Result, anyhow, bail, ensure};
use cairo_lang_executable::executable::{EntryPointKind, Executable};
use cairo_lang_runner::casm_run::format_for_panic;
use cairo_lang_runner::{Arg, CairoHintProcessor, build_hints_dict};
use cairo_lang_utils::bigint::BigUintAsHex;
use cairo_program_runner_lib::utils::get_cairo_run_config;
use cairo_program_runner_lib::{BootloaderHintProcessor, ProgramInput, PROGRAM_INPUT, PROGRAM_OBJECT};
use cairo_vm::Felt252;
use cairo_vm::cairo_run::cairo_run_program;
use cairo_vm::cairo_run::{CairoRunConfig, cairo_run_program_with_initial_scope};
use cairo_vm::types::exec_scope::ExecutionScopes;
use cairo_vm::types::layout_name::LayoutName;
use cairo_vm::types::program::Program;
use cairo_vm::types::relocatable::MaybeRelocatable;
use cairo_vm::vm::runners::cairo_runner::CairoRunner;
use camino::{Utf8Path, Utf8PathBuf};
use create_output_dir::create_output_dir;
use indoc::formatdoc;
use num_bigint::BigInt;
use scarb_extensions_cli::execute::{
    Args, BuildTargetSpecifier, ExecutionArgs, ExecutionTarget, OutputFormat, ProgramArguments,
};
use scarb_fs_utils::canonicalize_utf8;
use scarb_fs_utils::{MANIFEST_FILE_NAME, find_manifest_path};
use scarb_metadata::{Metadata, MetadataCommand, PackageMetadata, ScarbCommand, TargetMetadata};
use scarb_oracle_hint_service::OracleHintService;
use scarb_ui::Ui;
use scarb_ui::args::{PackagesFilter, ToEnvVars, WithManifestPath};
use scarb_ui::components::Status;
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::io::{self};
use std::str::FromStr;
use stwo_cairo_adapter::adapter::adapt;

mod hint_processor;
mod profiler;

pub(crate) mod output;

const MAX_ITERATION_COUNT: usize = 10000;
const EXECUTION_ID_ENV: &str = "SCARB_EXECUTION_ID";

const COMPILED_BOOTLOADER: &str = include_str!("../bootloaders/simple_bootloader.json");

pub fn main_inner(args: Args, ui: Ui) -> Result<()> {
    let metadata = MetadataCommand::new()
        .envs(args.execution.features.clone().to_env_vars())
        .inherit_stderr()
        .exec()?;
    let package = args.packages_filter.match_one(&metadata)?;
    execute(&metadata, &package, &args.execution, &ui)
}

fn read_arguments_as_felts(arguments: ProgramArguments) -> Result<Vec<Arg>> {
    if let Some(path) = arguments.arguments_file {
        let file = fs::File::open(&path).with_context(|| "reading arguments file failed")?;
        let as_vec: Vec<BigUintAsHex> =
            serde_json::from_reader(file).with_context(|| "deserializing arguments file failed")?;
        Ok(as_vec
            .into_iter()
            .map(|v| Arg::Value(v.value.into()))
            .collect())
    } else {
        Ok(arguments.arguments.into_iter().map(Arg::Value).collect())
    }
}

fn read_arguments_as_values(arguments: ProgramArguments) -> Result<Vec<Value>> {
    if let Some(path) = arguments.arguments_file {
        let file = fs::File::open(&path).with_context(|| "reading arguments file failed")?;
        let as_vec: Vec<BigUintAsHex> =
            serde_json::from_reader(file).with_context(|| "deserializing arguments file failed")?;
        Ok(as_vec
            .into_iter()
            .map(|v| {
                Value::Number(
                    serde_json::Number::from_str(&BigInt::from(v.value).to_string()).unwrap(),
                )
            })
            .collect())
    } else {
        Ok(arguments
            .arguments
            .into_iter()
            .map(|v| {
                Value::Number(serde_json::Number::from_str(&v.to_bigint().to_string()).unwrap())
            })
            .collect())
    }
}

fn execute_bootloader(
    executable_path: Utf8PathBuf,
    cairo_run_config: &CairoRunConfig,
    arguments: ProgramArguments,
) -> Result<(CairoRunner, Box<dyn ExecutionResourcesSource>)> {
    let args = read_arguments_as_values(arguments)?;

    // Program input JSON for the bootloader.
    let program_input = json!({
        "single_page": true,
        "tasks": [
            {
                "type": "Cairo1Executable",
                "path": executable_path,
                "program_hash_function": "blake",
                "user_args_list": args,
            }
        ],
    });
    let program_input = ProgramInput::Json(serde_json::to_string(&program_input)?);

    // Load bootloader program from embedded resource
    let bootloader_program = Program::from_bytes(COMPILED_BOOTLOADER.as_bytes(), Some("main"))
        .context("failed to load bootloader program")?;

    let mut hint_processor = BootloaderHintProcessor::new(None);

    let mut exec_scopes = ExecutionScopes::new();
    // Insert the program input into the execution scopes if exists
    exec_scopes.insert_value(PROGRAM_INPUT, program_input);
    // Insert the program object into the execution scopes
    exec_scopes.insert_value(PROGRAM_OBJECT, bootloader_program.clone());

    // Run the program with the configured execution scopes and cairo_run_config
    let runner = cairo_run_program_with_initial_scope(
        &bootloader_program,
        cairo_run_config,
        &mut hint_processor,
        exec_scopes,
    )
    .map_err(|err| {
        if let Some(cairo_hint_proc) = hint_processor.subtask_cairo1_hint_processor_stack.last()
            && let Some(panic_data) = cairo_hint_proc
                .as_ref()
                .and_then(|proc| proc.markers.last())
        {
            anyhow!(format_for_panic(panic_data.iter().copied()))
        } else {
            anyhow::Error::from(err).context("Cairo program run failed")
        }
    })?;

    Ok((runner, Box::new(hint_processor)))
}

fn execute_standalone(
    executable_path: Utf8PathBuf,
    executable: &Executable,
    cairo_run_config: &CairoRunConfig,
    arguments: ProgramArguments,
) -> Result<(CairoRunner, Box<dyn ExecutionResourcesSource>)> {
    let data = executable
        .program
        .bytecode
        .iter()
        .map(Felt252::from)
        .map(MaybeRelocatable::from)
        .collect();

    let (hints, string_to_hint) = build_hints_dict(&executable.program.hints);

    let entrypoint = executable
        .entrypoints
        .iter()
        .find(|e| matches!(e.kind, EntryPointKind::Standalone))
        .with_context(|| "no `Standalone` entrypoint found")?;
    let program = Program::new_for_proof(
        entrypoint.builtins.clone(),
        data,
        entrypoint.offset,
        entrypoint.offset + 4,
        hints,
        Default::default(),
        Default::default(),
        vec![],
        None,
    )
    .context("failed setting up program")?;

    let cairo_hint_processor = CairoHintProcessor {
        runner: None,
        user_args: vec![vec![Arg::Array(read_arguments_as_felts(arguments)?)]],
        string_to_hint,
        starknet_state: Default::default(),
        run_resources: Default::default(),
        syscalls_used_resources: Default::default(),
        no_temporary_segments: false,
        markers: Default::default(),
        panic_traceback: Default::default(),
    };

    let mut hint_processor = ExecuteHintProcessor {
        cairo_hint_processor,
        oracle_hint_service: OracleHintService::new(Some(executable_path.as_std_path())),
    };

    let runner =
        cairo_run_program(&program, cairo_run_config, &mut hint_processor).map_err(|err| {
            if let Some(panic_data) = hint_processor.cairo_hint_processor.markers.last() {
                anyhow!(format_for_panic(panic_data.iter().copied()))
            } else {
                anyhow::Error::from(err).context("Cairo program run failed")
            }
        })?;

    Ok((runner, Box::new(hint_processor)))
}

fn build_cairo_run_config(
    output: &OutputFormat,
    target: &ExecutionTarget,
    args: &ExecutionArgs,
) -> Result<CairoRunConfig<'static>> {
    let relocate_mem =
        output.is_standard() || args.run.print_resource_usage || args.run.save_profiler_trace_data;
    if target.is_bootloader() {
        Ok(get_cairo_run_config(
            &None,
            args.run.layout,
            true,
            true,
            true,
            relocate_mem,
        )?)
    } else {
        Ok(CairoRunConfig {
            allow_missing_builtins: Some(true),
            layout: args.run.layout,
            proof_mode: true,
            disable_trace_padding: true,
            fill_holes: true,
            secure_run: None,
            relocate_mem,
            trace_enabled: output.is_standard()
                || args.run.print_resource_usage
                || args.run.save_profiler_trace_data,
            ..Default::default()
        })
    }
}

pub fn execute(
    metadata: &Metadata,
    package: &PackageMetadata,
    args: &ExecutionArgs,
    ui: &Ui,
) -> Result<()> {
    let output = args
        .run
        .output
        .as_ref()
        .cloned()
        .unwrap_or(OutputFormat::None);
    let target = args
        .run
        .target
        .clone()
        .unwrap_or(ExecutionTarget::Standalone);
    output.validate(&target)?;

    if !args.no_build {
        let filter = PackagesFilter::generate_for::<Metadata>([package.clone()].iter());
        ScarbCommand::new()
            .arg("build")
            .env("SCARB_PACKAGES_FILTER", filter.to_env())
            .env("SCARB_UI_VERBOSITY", ui.verbosity().to_string())
            .envs(args.features.clone().to_env_vars())
            .run()?;
    }

    let scarb_target_dir = scarb_target_dir_from_env()?;
    let scarb_build_dir = scarb_target_dir
        .join(env::var("SCARB_PROFILE").context("`SCARB_PROFILE` env var must be defined")?);

    let build_target = find_build_target(metadata, package, &args.build_target_args)?;

    let syscalls_allowed = build_target
        .params
        .get("allow-syscalls")
        .and_then(|v| v.as_bool())
        .unwrap_or_default();
    if syscalls_allowed && args.run.layout == LayoutName::all_cairo_stwo {
        ui.warn(formatdoc!(r#"
            the executable target {} you are trying to execute has `allow-syscalls` set to `true`
            if your executable uses syscalls, it cannot be run with `all_cairo_stwo` layout
            please use `--layout` flag to specify a different layout, for example: `--layout=all_cairo`
        "#, build_target.name));
    }

    ui.print(Status::new("Executing", &package.name));
    let executable_path = find_prebuilt_executable_path(
        &scarb_build_dir,
        format!("{}.executable.json", build_target.name),
    )?;

    let executable = load_prebuilt_executable(&executable_path)?;

    let output_dir = scarb_target_dir.join("execute").join(&package.name);
    create_output_dir(output_dir.as_std_path())?;

    let execution_output_dir = get_or_create_output_dir(&output_dir)?;
    let cairo_run_config = build_cairo_run_config(&output, &target, args)?;

    let (mut runner, hint_processor) = if target.is_bootloader() {
        execute_bootloader(
            executable_path,
            &cairo_run_config,
            args.run.arguments.clone(),
        )
    } else {
        execute_standalone(
            executable_path,
            &executable,
            &cairo_run_config,
            args.run.arguments.clone(),
        )
    }?;

    let execution_resources = (args.run.print_resource_usage || args.run.save_profiler_trace_data)
        .then(|| ExecutionResources::try_new(&runner, hint_processor))
        .transpose()?;

    let summary = ExecutionSummary {
        program_output: args
            .run
            .print_program_output
            .then(|| ExecutionOutput::try_new(&mut runner))
            .transpose()?,
        resources: args
            .run
            .print_resource_usage
            .then_some(execution_resources.clone())
            .flatten(),
    };
    if args.run.print_resource_usage || args.run.print_program_output {
        ui.force_print(summary);
    } else {
        ui.print(summary);
    }

    if output.is_cairo_pie() {
        let output_value = runner.get_cairo_pie()?;
        let output_file_path = execution_output_dir.join("cairo_pie.zip");
        ui.print(Status::new(
            "Saving output to:",
            &display_path(&scarb_target_dir, &output_file_path),
        ));
        output_value.write_zip_file(output_file_path.as_std_path(), true)?;
    } else if output.is_standard() {
        ui.print(Status::new(
            "Saving output to:",
            &display_path(&scarb_target_dir, &execution_output_dir),
        ));

        let adapted = adapt(&runner)?;
        let input_path = execution_output_dir.join("prover_input.json");
        fs::write(input_path, serde_json::to_string(&adapted)?)?;
    }

    if args.run.save_profiler_trace_data {
        ensure!(
            build_target.params.get("sierra").and_then(|v| v.as_bool()) == Some(true),
            "Failed to write profiler trace data into a file — missing sierra code for target `{0}`. \
            Set `sierra = true` under your `[executable]` target in the config and try again.",
            build_target.name
        );
        let executable_sierra_path = scarb_build_dir
            .join(&build_target.name)
            .with_extension("executable.sierra.json");
        ensure!(
            executable_sierra_path.exists(),
            "Missing sierra code for executable `{0}`, file {executable_sierra_path} does not exist. \
             help: run `scarb build` to compile the package and try again.",
            build_target.name
        );
        let tracked_resource = get_profiler_tracked_resource(package)?;
        let function_name: Option<String> = build_target
            .params
            .get("function")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        let program_offset = executable
            .debug_info
            .as_ref()
            .expect("Missing debug info in executable")
            .annotations
            .get("github.com/software-mansion/cairo-profiler")
            .and_then(|v| v.get("program_info"))
            .and_then(|v| v.get("program_offset"))
            .and_then(|v| v.as_u64())
            .expect("Missing or invalid program_offset in debug info")
            as usize;
        let call_trace = build_profiler_call_trace(
            &target,
            runner.relocated_trace.clone(),
            execution_resources.expect("Failed to obtain execution resources"),
            &tracked_resource,
            executable_sierra_path,
            function_name,
            program_offset,
        )?;
        ui.print(Status::new(
            "Profiler tracked resource:",
            tracked_resource.into(),
        ));

        // Write profiler trace file
        let profiler_trace_path = execution_output_dir.join("cairo_profiler_trace.json");
        ui.print(Status::new(
            "Saving profiler trace data to:",
            profiler_trace_path.as_ref(),
        ));
        let serialized_trace = serde_json::to_string(&call_trace)
            .expect("Failed to serialize call trace for profiler");
        fs::write(profiler_trace_path, serialized_trace)?;
    }

    Ok(())
}

fn scarb_target_dir_from_env() -> Result<Utf8PathBuf> {
    match env::var("SCARB_TARGET_DIR") {
        Ok(value) => Ok(Utf8PathBuf::from(value)),
        Err(_) => {
            let manifest_path = find_manifest_path(None)?;
            if manifest_path.exists() {
                bail!("`SCARB_TARGET_DIR` env var must be defined")
            } else {
                bail!(
                    "no {MANIFEST_FILE_NAME} found, this command must be run inside a Scarb project"
                )
            }
        }
    }
}

fn find_build_target<'a>(
    metadata: &Metadata,
    package: &'a PackageMetadata,
    build_target_args: &BuildTargetSpecifier,
) -> Result<&'a TargetMetadata> {
    let executable_targets = package
        .targets
        .iter()
        .filter(|target| target.kind.as_str() == "executable")
        .collect::<Vec<_>>();

    ensure!(
        !executable_targets.is_empty(),
        missing_executable_target_error(metadata, package)
    );

    let matched_by_args = executable_targets.iter().find(|target| {
        let build_target_function = target
            .params
            .as_object()
            .and_then(|params| params.get("function"))
            .and_then(|x| x.as_str());
        let function_matches = build_target_function
            .is_some_and(|left| Some(left) == build_target_args.executable_function.as_deref());
        let name_matches = build_target_args
            .executable_name
            .as_deref()
            .is_some_and(|name| target.name == name);
        name_matches || function_matches
    });

    if let Some(matched) = matched_by_args {
        return Ok(matched);
    }

    // `--executable-name` and `--executable-function` names have not matched any target.
    if let Some(name) = build_target_args.executable_name.as_deref() {
        bail!(
            "no executable target with name `{name}` found for package `{}`",
            package.name
        )
    }
    if let Some(function) = build_target_args.executable_function.as_deref() {
        bail!(
            "no executable target with executable function `{function}` found for package `{}`",
            package.name
        )
    }

    ensure!(
        executable_targets.len() == 1,
        formatdoc! {r#"
            more than one executable target found for package `{}`
            help: specify the target with `--executable-name` or `--executable-function`
            "#, package.name
        }
    );

    Ok(executable_targets[0])
}

fn missing_executable_target_error(metadata: &Metadata, package: &PackageMetadata) -> String {
    let scarb_version = metadata
        .app_version_info
        .clone()
        .version
        .clone()
        .to_string();
    let scarb_toml = package.manifest_path.clone();
    let scarb_toml = scarb_toml
        .strip_prefix(metadata.workspace.root.clone())
        .unwrap_or_else(|_| package.manifest_path());
    formatdoc! {r#"
        no executable target found for package `{}`
        help: you can add `executable` target to the package manifest with following excerpt
        -> {scarb_toml}
            [executable]

            [dependencies]
            cairo_execute = "{scarb_version}"
        "#
    , package.name}
}

fn display_path(scarb_target_dir: &Utf8Path, output_path: &Utf8Path) -> String {
    Utf8PathBuf::from("target")
        .join(
            output_path
                .strip_prefix(scarb_target_dir)
                .unwrap_or(output_path),
        )
        .to_string()
}

fn find_prebuilt_executable_path(path: &Utf8Path, filename: String) -> Result<Utf8PathBuf> {
    let file_path = path.join(&filename);
    ensure!(
        file_path.exists(),
        formatdoc! {r#"
            package has not been compiled, file does not exist: `{filename}`
            help: run `scarb build` to compile the package
        "#}
    );

    let file_path =
        canonicalize_utf8(file_path).context("failed to canonicalize executable path")?;
    Ok(file_path)
}

fn load_prebuilt_executable(file_path: &Utf8Path) -> Result<Executable> {
    let file = fs::File::open(file_path)
        .with_context(|| format!("failed to open executable program: `{file_path}`"))?;
    serde_json::from_reader(file)
        .with_context(|| format!("failed to deserialize executable program: `{file_path}`"))
}

fn get_or_create_output_dir(output_dir: &Utf8Path) -> Result<Utf8PathBuf> {
    if let Some(execution_id) = env::var_os(EXECUTION_ID_ENV) {
        let execution_id: usize = execution_id
            .to_string_lossy()
            .parse()
            .map_err(|_| anyhow!("invalid execution id in environment variable"))?;
        let execution_output_dir = output_dir.join(format!("execution{execution_id}"));
        ensure!(
            execution_output_dir.exists(),
            "execution output directory does not exist"
        );
        return Ok(execution_output_dir);
    }
    incremental_create_output_dir(output_dir)
}

fn incremental_create_output_dir(path: &Utf8Path) -> Result<Utf8PathBuf> {
    for i in 1..=MAX_ITERATION_COUNT {
        let filepath = path.join(format!("execution{i}"));
        let result = fs::create_dir(&filepath);
        return match result {
            Err(e) => {
                if e.kind() == io::ErrorKind::AlreadyExists {
                    continue;
                }
                Err(e.into())
            }
            Ok(_) => Ok(filepath),
        };
    }
    bail!("failed to create output directory")
}
