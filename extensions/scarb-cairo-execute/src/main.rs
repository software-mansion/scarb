use anyhow::{bail, ensure, Context, Result};
use bincode::enc::write::Writer;
use cairo_lang_executable::executable::{EntryPointKind, Executable};
use cairo_lang_runner::{build_hints_dict, Arg, CairoHintProcessor};
use cairo_vm::cairo_run::cairo_run_program;
use cairo_vm::cairo_run::CairoRunConfig;
use cairo_vm::types::layout_name::LayoutName;
use cairo_vm::types::program::Program;
use cairo_vm::types::relocatable::MaybeRelocatable;
use cairo_vm::{cairo_run, Felt252};
use camino::Utf8PathBuf;
use clap::{arg, Parser, ValueEnum};
use create_output_dir::create_output_dir;
use indoc::formatdoc;
use num_bigint::BigInt;
use scarb::core::TargetKind;
use scarb_metadata::{Metadata, MetadataCommand, ScarbCommand};
use scarb_ui::args::{PackagesFilter, VerbositySpec};
use scarb_ui::components::Status;
use scarb_ui::Ui;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const BASE_OUTPUT_DIRECTORY: &str = "target/cairo-execute";
const EXECUTION_FILE: (&str, &str) = ("execution", ".zip");
const AIR_PUBLIC_FILE: (&str, &str) = ("air_public", ".json");
const AIR_PRIVATE_FILE: (&str, &str) = ("air_private", ".json");
const MEMORY_FILENAME: &str = "memory.json";
const TRACE_FILENAME: &str = "trace.json";
const MAX_ITERATION_COUNT: usize = 10000;

#[derive(ValueEnum, Clone, Debug)]
enum OutputFormat {
    CairoPie,
    Standard,
}
impl OutputFormat {
    pub fn is_cairo_pie(&self) -> bool {
        matches!(self, OutputFormat::CairoPie)
    }
}

/// Compiles a Cairo project and runs a function marked `#[executable]`.
/// Exits with 1 if the compilation or run fails, otherwise 0.
#[derive(Parser, Clone, Debug)]
#[clap(version, verbatim_doc_comment)]
struct Args {
    /// Name of the package.
    #[command(flatten)]
    packages_filter: PackagesFilter,

    /// Do not rebuild the package.
    #[arg(long, default_value_t = false)]
    no_build: bool,

    #[clap(flatten)]
    run: ExecutionArgs,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
}

#[derive(Parser, Clone, Debug)]
struct ExecutionArgs {
    /// Desired execution output, either default Standard or CairoPie
    #[arg(short, long)]
    pub output: Option<OutputFormat>,

    /// Serialized arguments to the executable function.
    #[arg(long, value_delimiter = ',')]
    arguments: Vec<BigInt>,

    /// Whether to print the outputs.
    #[arg(long, default_value_t = false)]
    print_outputs: bool,

    /// If set, the program will be run in proof mode.
    #[clap(long, default_value_t = false)]
    proof_mode: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let ui = Ui::new(args.verbose.clone().into(), scarb_ui::OutputFormat::Text);

    match main_inner(args, ui.clone()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            ui.error(format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}

fn main_inner(args: Args, ui: Ui) -> Result<(), anyhow::Error> {
    let metadata = MetadataCommand::new().inherit_stderr().exec()?;
    let package = args
        .packages_filter
        .match_one(&metadata)
        .map_err(|e| anyhow::anyhow!("Failed to match package in workspace: {e:?}"))?;

    if !args.no_build {
        let filter = PackagesFilter::generate_for::<Metadata>(vec![package.clone()].iter());
        ScarbCommand::new()
            .arg("build")
            .env("SCARB_PACKAGES_FILTER", filter.to_env())
            .run()?;
    }

    let filename = format!("{}.{}.json", package.name, TargetKind::EXECUTABLE);
    let path = Utf8PathBuf::from(env::var("SCARB_TARGET_DIR")?).join(env::var("SCARB_PROFILE")?);

    ui.print(Status::new("Executing", &package.name));
    let executable = load_prebuilt_executable(&path, filename)?;

    let data = executable
        .program
        .bytecode
        .iter()
        .map(Felt252::from)
        .map(MaybeRelocatable::from)
        .collect();

    let (hints, string_to_hint) = build_hints_dict(&executable.program.hints);

    let program = if args.run.proof_mode {
        let entrypoint = executable
            .entrypoints
            .iter()
            .find(|e| matches!(e.kind, EntryPointKind::Standalone))
            .with_context(|| "No `Standalone` entrypoint found.")?;
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
            .with_context(|| "No `Bootloader` entrypoint found.")?;
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
    .with_context(|| "Failed setting up program.")?;

    let mut hint_processor = CairoHintProcessor {
        runner: None,
        user_args: vec![vec![Arg::Array(
            args.run
                .arguments
                .iter()
                .map(|v| Arg::Value(v.into()))
                .collect(),
        )]],
        string_to_hint,
        starknet_state: Default::default(),
        run_resources: Default::default(),
        syscalls_used_resources: Default::default(),
        no_temporary_segments: false,
    };

    let cairo_run_config = CairoRunConfig {
        trace_enabled: true,
        relocate_mem: false,
        layout: LayoutName::all_cairo,
        proof_mode: args.run.proof_mode,
        secure_run: None,
        allow_missing_builtins: Some(true),
        ..Default::default()
    };

    let mut runner = cairo_run_program(&program, &cairo_run_config, &mut hint_processor)
        .map_err(|e| anyhow::anyhow!("Cairo program run failed: {e:?}"))?;

    if args.run.print_outputs {
        let mut output_buffer = "Program Output:\n".to_string();
        runner.vm.write_output(&mut output_buffer)?;
        print!("{output_buffer}");
    }

    let directory_path = Path::new(BASE_OUTPUT_DIRECTORY);
    create_output_dir(directory_path)?;
    let base_path = create_execution_directory(directory_path)?;

    if !args
        .run
        .output
        .unwrap_or(OutputFormat::Standard)
        .is_cairo_pie()
    {
        let trace_file_path = base_path.join(TRACE_FILENAME);

        let relocated_trace = runner
            .relocated_trace
            .as_ref()
            .with_context(|| "Trace not relocated.")?;
        let mut writer = FileWriter::new(3 * 1024 * 1024, &trace_file_path)?;
        cairo_run::write_encoded_trace(relocated_trace, &mut writer)?;
        writer.flush()?;

        let memory_file_path = base_path.join(MEMORY_FILENAME);
        let mut writer = FileWriter::new(5 * 1024 * 1024, &memory_file_path)?;
        cairo_run::write_encoded_memory(&runner.relocated_memory, &mut writer)?;
        writer.flush()?;

        let air_public_file_path =
            create_incremental_file_in_dir(&base_path, Some(AIR_PUBLIC_FILE.1), AIR_PUBLIC_FILE.0)?;
        let json = runner.get_air_public_input()?.serialize_json()?;
        fs::write(&air_public_file_path, json)?;

        let air_private_file_path = create_incremental_file_in_dir(
            &base_path,
            Some(AIR_PRIVATE_FILE.1),
            AIR_PRIVATE_FILE.0,
        )?;
        let absolute = |path_buf: PathBuf| {
            path_buf
                .as_path()
                .canonicalize()
                .unwrap_or(path_buf)
                .to_string_lossy()
                .to_string()
        };
        let json = runner
            .get_air_private_input()
            .to_serializable(absolute(trace_file_path), absolute(memory_file_path))
            .serialize_json()
            .with_context(|| "Failed serializing private input")?;
        fs::write(air_private_file_path, json)?
    } else {
        let file_path =
            create_incremental_file_in_dir(&base_path, Some(EXECUTION_FILE.1), EXECUTION_FILE.0)?;
        runner.get_cairo_pie()?.write_zip_file(&file_path)?
    }
    Ok(())
}

fn load_prebuilt_executable(path: &Utf8PathBuf, filename: String) -> anyhow::Result<Executable> {
    let file_path = path.join(&filename);
    ensure!(
        file_path.exists(),
        formatdoc! {r#"
            package has not been compiled, file does not exist: {filename}
            help: run `scarb build` to compile the package
        "#}
    );
    let file = fs::File::open(&file_path)
        .with_context(|| format!("failed to open executable program: {file_path}"))?;
    serde_json::from_reader(file)
        .with_context(|| format!("failed to deserialize executable program: {file_path}"))
}

fn create_execution_directory(base_path: &Path) -> Result<PathBuf, anyhow::Error> {
    let mut counter = 1;
    loop {
        let dir_name = format!("execution{}", counter);
        let dir_path = base_path.join(dir_name);

        if !dir_path.exists() {
            create_output_dir(dir_path.as_path())?;
            return Ok(dir_path);
        }
        if counter > MAX_ITERATION_COUNT {
            bail!("failed to create execution directory, max iteration count reached");
        }
        counter += 1;
    }
}

fn create_incremental_file_in_dir(
    directory_path: &Path,
    extension: Option<&str>,
    filename: &str,
) -> Result<PathBuf, anyhow::Error> {
    let extension = extension.unwrap_or(".zip");
    let filepath = directory_path.join(format!("{}{}", filename, extension));
    fs::File::create(&filepath)?;
    Ok(filepath)
}

/// Writer implementation for a file.
struct FileWriter {
    buf_writer: io::BufWriter<std::fs::File>,
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
    fn new(capacity: usize, path: &PathBuf) -> anyhow::Result<Self> {
        Ok(Self {
            buf_writer: io::BufWriter::with_capacity(capacity, std::fs::File::create(path)?),
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
