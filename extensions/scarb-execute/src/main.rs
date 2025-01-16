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
use camino::{Utf8Path, Utf8PathBuf};
use clap::{arg, Parser, ValueEnum};
use create_output_dir::create_output_dir;
use indoc::formatdoc;
use num_bigint::BigInt;
use scarb_metadata::{Metadata, MetadataCommand, ScarbCommand};
use scarb_ui::args::{PackagesFilter, VerbositySpec};
use scarb_ui::components::Status;
use scarb_ui::Ui;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::process::ExitCode;

const MAX_ITERATION_COUNT: usize = 10000;

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
    /// Serialized arguments to the executable function.
    #[arg(long, value_delimiter = ',')]
    arguments: Vec<BigInt>,

    /// Desired execution output, either default Standard or CairoPie
    #[arg(long, default_value = "standard")]
    pub output: OutputFormat,

    /// Execution target.
    #[arg(long, default_value = "standalone")]
    target: ExecutionTarget,

    /// Whether to print the program outputs.
    #[arg(long, default_value_t = false)]
    print_program_output: bool,
}

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

#[derive(ValueEnum, Clone, Debug)]
enum ExecutionTarget {
    Bootloader,
    Standalone,
}

impl ExecutionTarget {
    pub fn is_standalone(&self) -> bool {
        matches!(self, ExecutionTarget::Standalone)
    }
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
    ensure!(
        !(args.run.output.is_cairo_pie() && args.run.target.is_standalone()),
        "Cairo pie output format is not supported for standalone execution target"
    );

    let metadata = MetadataCommand::new().inherit_stderr().exec()?;
    let package = args.packages_filter.match_one(&metadata)?;

    if !args.no_build {
        let filter = PackagesFilter::generate_for::<Metadata>(vec![package.clone()].iter());
        ScarbCommand::new()
            .arg("build")
            .env("SCARB_PACKAGES_FILTER", filter.to_env())
            .run()?;
    }

    let scarb_target_dir = Utf8PathBuf::from(env::var("SCARB_TARGET_DIR")?);
    let scarb_build_dir = scarb_target_dir.join(env::var("SCARB_PROFILE")?);

    ui.print(Status::new("Executing", &package.name));
    let executable = load_prebuilt_executable(
        &scarb_build_dir,
        format!("{}.executable.json", package.name),
    )?;

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
        allow_missing_builtins: Some(true),
        layout: LayoutName::all_cairo,
        proof_mode: args.run.target.is_standalone(),
        relocate_mem: args.run.target.is_standalone(),
        secure_run: None,
        trace_enabled: true,
        ..Default::default()
    };

    let mut runner = cairo_run_program(&program, &cairo_run_config, &mut hint_processor)
        .with_context(|| "Cairo program run failed")?;

    if args.run.print_program_output {
        let mut output_buffer = "Program Output:\n".to_string();
        runner.vm.write_output(&mut output_buffer)?;
        print!("{output_buffer}");
    }

    let output_dir = scarb_target_dir.join("scarb-execute");
    create_output_dir(output_dir.as_std_path())?;

    if args.run.output.is_cairo_pie() {
        let output_value = runner.get_cairo_pie()?;
        let output_file_path = incremental_create_output_file(&output_dir, package.name, ".zip")?;
        ui.print(Status::new(
            "Saving output to:",
            &display_path(&scarb_target_dir, &output_file_path),
        ));
        output_value.write_zip_file(output_file_path.as_std_path())?;
    } else {
        let execution_output_dir = incremental_create_output_dir(&output_dir, package.name)?;
        ui.print(Status::new(
            "Saving output to:",
            &display_path(&scarb_target_dir, &execution_output_dir),
        ));

        // Write trace file.
        let trace_path = execution_output_dir.join("trace.bin");
        let relocated_trace = runner
            .relocated_trace
            .as_ref()
            .with_context(|| "Trace not relocated.")?;
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
            .with_context(|| "Failed serializing private input.")?;
        fs::write(air_private_input_path, output_value)?;
    }

    Ok(())
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

fn load_prebuilt_executable(path: &Utf8Path, filename: String) -> Result<Executable> {
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

fn incremental_create_output_file(
    path: &Utf8Path,
    name: String,
    extension: impl AsRef<str>,
) -> Result<Utf8PathBuf> {
    incremental_attempt_io_creation(
        path,
        name,
        extension,
        "Failed to create output directory.",
        |p| {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(p)
                .map(|_| ())
        },
    )
}

fn incremental_create_output_dir(path: &Utf8Path, name: String) -> Result<Utf8PathBuf> {
    incremental_attempt_io_creation(path, name, "", "Failed to create output directory.", |p| {
        fs::create_dir(p)
    })
}

fn incremental_attempt_io_creation(
    path: &Utf8Path,
    name: impl AsRef<str>,
    extension: impl AsRef<str>,
    final_error_message: impl AsRef<str>,
    attempt: impl Fn(&Utf8Path) -> io::Result<()>,
) -> Result<Utf8PathBuf> {
    for i in 0..MAX_ITERATION_COUNT {
        let number_string = if i == 0 {
            "".to_string()
        } else {
            format!("_{}", i)
        };
        let filepath = path.join(format!(
            "{}{}{}",
            name.as_ref(),
            number_string,
            extension.as_ref()
        ));
        let result = attempt(&filepath);
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
    bail!(final_error_message.as_ref().to_string());
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
