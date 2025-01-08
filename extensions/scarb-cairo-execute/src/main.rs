use anyhow::{ensure, Result};
use cairo_lang_executable::executable::{EntryPointKind, Executable};
use cairo_lang_runner::{build_hints_dict, Arg, CairoHintProcessor};
use cairo_vm::cairo_run::cairo_run_program;
use cairo_vm::cairo_run::CairoRunConfig;
use cairo_vm::types::layout_name::LayoutName;
use cairo_vm::types::program::Program;
use cairo_vm::types::relocatable::MaybeRelocatable;
use cairo_vm::Felt252;
use camino::Utf8PathBuf;
use clap::Parser;
use create_output_dir::create_output_dir;
use indoc::formatdoc;
use num_bigint::BigInt;
use scarb_metadata::MetadataCommand;
use scarb_ui::args::{PackagesFilter, VerbositySpec};
use scarb_ui::components::Status;
use scarb_ui::Ui;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Compiles a Cairo project and runs a function marked `#[executable]`.
/// Exits with 1 if the compilation or run fails, otherwise 0.
#[derive(Parser, Debug)]
#[clap(version, verbatim_doc_comment)]
struct Args {
    /// Name of the package.
    #[command(flatten)]
    packages_filter: PackagesFilter,

    /// Whether to only run a prebuilt executable.
    #[arg(long, default_value_t = true)]
    prebuilt: bool,

    #[clap(flatten)]
    run: RunArgs,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
}

#[derive(Parser, Debug)]
struct RunArgs {
    /// Serialized arguments to the executable function.
    #[arg(long, value_delimiter = ',')]
    args: Vec<BigInt>,

    /// Whether to print the outputs.
    #[arg(long, default_value_t = false)]
    print_outputs: bool,

    #[clap(long)]
    cairo_pie_output: Option<PathBuf>,
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

    let filename = format!("{}.executable.json", package.name);
    let path = Utf8PathBuf::from(env::var("SCARB_TARGET_DIR")?).join(env::var("SCARB_PROFILE")?);

    ui.print(Status::new("Running", &package.name));
    let executable = load_prebuilt_executable(&path, filename)?;

    let data = executable
        .program
        .bytecode
        .iter()
        .map(Felt252::from)
        .map(MaybeRelocatable::from)
        .collect();

    let (hints, string_to_hint) = build_hints_dict(&executable.program.hints);

    let program = {
        let entrypoint = executable
            .entrypoints
            .iter()
            .find(|e| matches!(e.kind, EntryPointKind::Bootloader))
            .ok_or_else(|| anyhow::anyhow!("Bootloader entrypoint not found"))?;
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
    };

    let mut hint_processor = CairoHintProcessor {
        runner: None,
        user_args: vec![vec![Arg::Array(
            args.run.args.iter().map(|v| Arg::Value(v.into())).collect(),
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
        proof_mode: false,
        secure_run: Some(true),
        allow_missing_builtins: Some(true),
        ..Default::default()
    };

    let mut runner = cairo_run_program(&program?, &cairo_run_config, &mut hint_processor)
        .map_err(|e| anyhow::anyhow!("Cairo program run failed: {e:?}"))?;

    if args.run.print_outputs {
        let mut output_buffer = "Program Output:\n".to_string();
        runner.vm.write_output(&mut output_buffer)?;
        print!("{output_buffer}");
    }

    let file_path = create_incremental_file_in_dir("target/cairo-execute", Some(".zip"))?;
    runner.get_cairo_pie()?.write_zip_file(&file_path)?;

    Ok(())
}

fn load_prebuilt_executable(
    path: &Utf8PathBuf,
    filename: String,
) -> Result<Executable, anyhow::Error> {
    let file_path = path.join(&filename);
    ensure!(
        file_path.exists(),
        formatdoc! {r#"
            package has not been compiled, file does not exist: {filename}
            help: run `scarb build` to compile the package
        "#}
    );
    let file = std::fs::File::open(file_path)
        .map_err(|e| anyhow::anyhow!("Failed to open file: {}", e))?;
    serde_json::from_reader(file)
        .map_err(|e| anyhow::anyhow!("Failed parsing prebuilt executable: {}", e))
}

fn create_incremental_file_in_dir(
    directory: &str,
    extension: Option<&str>,
) -> Result<PathBuf, anyhow::Error> {
    let extension = extension.unwrap_or(".zip");
    let mut counter = 1;
    let directory_path = Path::new(directory);
    create_output_dir(directory_path)?;

    loop {
        let filepath = directory_path.join(format!("execution{}{}", counter, extension));
        if !filepath.exists() {
            fs::File::create(&filepath)?;
            return Ok(filepath);
        }
        counter += 1;
    }
}
