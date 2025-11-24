#![deny(clippy::dbg_macro)]
#![deny(clippy::disallowed_methods)]

use crate::hint_processor::ExecuteHintProcessor;
use crate::output::{ExecutionOutput, ExecutionResources, ExecutionSummary};
use crate::profiler::{build_profiler_call_trace, get_profiler_tracked_resource};
use anyhow::{Context, Result, anyhow, bail, ensure};
use bincode::enc::write::Writer;
use cairo_lang_executable::executable::{EntryPointKind, Executable};
use cairo_lang_runner::casm_run::{format_for_debug, format_for_panic};
use cairo_lang_runner::{Arg, CairoHintProcessor, build_hints_dict};
use cairo_lang_utils::bigint::BigUintAsHex;
use cairo_vm::cairo_run::CairoRunConfig;
use cairo_vm::cairo_run::cairo_run_program;
use cairo_vm::types::layout_name::LayoutName;
use cairo_vm::types::program::Program;
use cairo_vm::types::relocatable::MaybeRelocatable;
use cairo_vm::{Felt252, cairo_run};
use camino::{Utf8Path, Utf8PathBuf};
use create_output_dir::{create_output_dir, EXECUTE_PROGRAM_OUTPUT_FILENAME, EXECUTE_STDOUT_OUTPUT_FILENAME};
use indoc::formatdoc;
use scarb_extensions_cli::execute::{
    Args, BuildTargetSpecifier, ExecutionArgs, OutputFormat, ProgramArguments,
};
use scarb_metadata::{Metadata, MetadataCommand, PackageMetadata, ScarbCommand, TargetMetadata};
use scarb_oracle_hint_service::OracleHintService;
use scarb_ui::Ui;
use scarb_ui::args::{PackagesFilter, ToEnvVars, WithManifestPath};
use scarb_ui::components::Status;
use std::env;
use std::fs;
use std::io::{self, Write};

mod hint_processor;
mod profiler;

pub(crate) mod output;

const MAX_ITERATION_COUNT: usize = 10000;
const EXECUTION_ID_ENV: &str = "SCARB_EXECUTION_ID";

pub fn main_inner(args: Args, ui: Ui) -> Result<()> {
    let metadata = MetadataCommand::new()
        .envs(args.execution.features.clone().to_env_vars())
        .inherit_stderr()
        .exec()?;
    let package = args.packages_filter.match_one(&metadata)?;
    execute(&metadata, &package, &args.execution, &ui)
}

fn read_arguments(arguments: ProgramArguments) -> Result<Vec<Arg>> {
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
        .unwrap_or_else(|| OutputFormat::default_for_target(args.run.target.clone()));
    output.validate(&args.run.target)?;

    if !args.no_build {
        let filter = PackagesFilter::generate_for::<Metadata>([package.clone()].iter());
        ScarbCommand::new()
            .arg("build")
            .env("SCARB_PACKAGES_FILTER", filter.to_env())
            .env("SCARB_UI_VERBOSITY", ui.verbosity().to_string())
            .envs(args.features.clone().to_env_vars())
            .run()?;
    }

    let scarb_target_dir = Utf8PathBuf::from(env::var("SCARB_TARGET_DIR")?);
    let scarb_build_dir = scarb_target_dir.join(env::var("SCARB_PROFILE")?);

    let build_target = find_build_target(metadata, package, &args.build_target_args)?;

    ui.print(Status::new("Executing", &package.name));
    let executable_path = find_prebuilt_executable_path(
        &scarb_build_dir,
        format!("{}.executable.json", build_target.name),
    )?;
    let executable = load_prebuilt_executable(&executable_path)?;

    let data = executable
        .program
        .bytecode
        .iter()
        .map(Felt252::from)
        .map(MaybeRelocatable::from)
        .collect();

    let (hints, string_to_hint) = build_hints_dict(&executable.program.hints);

    let program = if args.run.target.is_standalone() {
        let entrypoint = executable
            .entrypoints
            .iter()
            .find(|e| matches!(e.kind, EntryPointKind::Standalone))
            .with_context(|| "no `Standalone` entrypoint found")?;
        Program::new_for_proof(
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
    } else {
        let entrypoint = executable
            .entrypoints
            .iter()
            .find(|e| matches!(e.kind, EntryPointKind::Bootloader))
            .with_context(|| "no `Bootloader` entrypoint found")?;
        Program::new(
            entrypoint.builtins.clone(),
            data,
            Some(entrypoint.offset),
            hints,
            Default::default(),
            Default::default(),
            vec![],
            None,
        )
    }
    .with_context(|| "failed setting up program")?;

    let cairo_hint_processor = CairoHintProcessor {
        runner: None,
        user_args: vec![vec![Arg::Array(read_arguments(
            args.run.arguments.clone(),
        )?)]],
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
        captured_print_felts: Vec::new(),
        capture_enabled: args.run.save_stdout_output,
    };

    let proof_mode = args.run.target.is_standalone();

    let cairo_run_config = CairoRunConfig {
        allow_missing_builtins: Some(true),
        layout: LayoutName::all_cairo,
        proof_mode,
        secure_run: None,
        relocate_mem: output.is_standard()
            || args.run.print_resource_usage
            || args.run.save_profiler_trace_data,
        trace_enabled: output.is_standard()
            || args.run.print_resource_usage
            || args.run.save_profiler_trace_data,
        disable_trace_padding: proof_mode,
        ..Default::default()
    };

    let mut runner =
        cairo_run_program(&program, &cairo_run_config, &mut hint_processor).map_err(|err| {
            if let Some(panic_data) = hint_processor.cairo_hint_processor.markers.last() {
                anyhow!(format_for_panic(panic_data.iter().copied()))
            } else {
                anyhow::Error::from(err).context("Cairo program run failed")
            }
        })?;

    let execution_resources = (args.run.print_resource_usage || args.run.save_profiler_trace_data)
        .then(|| ExecutionResources::try_new(&runner, hint_processor.cairo_hint_processor).ok())
        .flatten();

    let captured_print_output = if !hint_processor.captured_print_felts.is_empty() {
        format_for_debug(hint_processor.captured_print_felts.into_iter())
    } else {
        String::new()
    };

    let execution_output = (args.run.print_program_output || args.run.save_program_output)
        .then(|| ExecutionOutput::try_new(&mut runner))
        .transpose()?;

    ui.print(ExecutionSummary {
        output: if args.run.print_program_output {
            execution_output.clone()
        } else {
            None
        },
        resources: args
            .run
            .print_resource_usage
            .then_some(execution_resources.clone())
            .flatten(),
    });

    if output.is_none() {
        return Ok(());
    }

    let output_dir = scarb_target_dir.join("execute").join(&package.name);
    create_output_dir(output_dir.as_std_path())?;

    let execution_output_dir = get_or_create_output_dir(&output_dir)?;

    if output.is_cairo_pie() {
        let output_value = runner.get_cairo_pie()?;
        let output_file_path = execution_output_dir.join("cairo_pie.zip");
        ui.print(Status::new(
            "Saving output to:",
            &display_path(&scarb_target_dir, &output_file_path),
        ));
        output_value.write_zip_file(output_file_path.as_std_path(), true)?;
    } else {
        ui.print(Status::new(
            "Saving output to:",
            &display_path(&scarb_target_dir, &execution_output_dir),
        ));

        // Write trace file.
        let trace_path = execution_output_dir.join("trace.bin");
        let relocated_trace = runner
            .relocated_trace
            .as_ref()
            .with_context(|| "trace not relocated")?;
        let mut writer = FileWriter::new(3 * 1024 * 1024, &trace_path)?;
        cairo_run::write_encoded_trace(relocated_trace, &mut writer)?;
        writer.flush()?;

        // Write memory file.
        let memory_path = execution_output_dir.join("memory.bin");
        let mut writer = FileWriter::new(5 * 1024 * 1024, &memory_path)?;
        cairo_run::write_encoded_memory(&runner.relocated_memory, &mut writer)?;
        writer.flush()?;

        // Write air public input file.
        let air_public_input_path = execution_output_dir.join("air_public_input.json");
        let json = runner.get_air_public_input()?.serialize_json()?;
        fs::write(air_public_input_path, json)?;

        // Write air private input file.
        let air_private_input_path = execution_output_dir.join("air_private_input.json");
        let output_value = runner
            .get_air_private_input()
            .to_serializable(trace_path.to_string(), memory_path.to_string())
            .serialize_json()
            .with_context(|| "failed serializing private input")?;
        fs::write(air_private_input_path, output_value)?;
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
            &args.run.target,
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

    if args.run.save_program_output
        && let Some(output) = &execution_output
    {
        let program_output_path = execution_output_dir.join(EXECUTE_PROGRAM_OUTPUT_FILENAME);
        fs::write(program_output_path, output.as_str())?;
    }

    if args.run.save_stdout_output && !captured_print_output.is_empty() {
        let stdout_output_path = execution_output_dir.join(EXECUTE_STDOUT_OUTPUT_FILENAME);
        fs::write(stdout_output_path, &captured_print_output)?;
    }

    Ok(())
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

/// Writer implementation for a file.
struct FileWriter {
    buf_writer: io::BufWriter<fs::File>,
    bytes_written: usize,
}

impl Writer for FileWriter {
    fn write(&mut self, bytes: &[u8]) -> Result<(), bincode::error::EncodeError> {
        self.buf_writer
            .write_all(bytes)
            .map_err(|e| bincode::error::EncodeError::Io {
                inner: e,
                index: self.bytes_written,
            })?;

        self.bytes_written += bytes.len();

        Ok(())
    }
}

impl FileWriter {
    /// Create a new instance of `FileWriter` with the given file path.
    fn new(capacity: usize, path: &Utf8Path) -> Result<Self> {
        Ok(Self {
            buf_writer: io::BufWriter::with_capacity(capacity, fs::File::create(path)?),
            bytes_written: 0,
        })
    }

    /// Flush the writer.
    ///
    /// Would automatically be called when the writer is dropped, but errors are ignored in that
    /// case.
    fn flush(&mut self) -> io::Result<()> {
        self.buf_writer.flush()
    }
}
