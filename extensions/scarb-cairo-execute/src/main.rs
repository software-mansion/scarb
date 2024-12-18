use anyhow::{ensure, Result};
use cairo_lang_executable::executable::{EntryPointKind, Executable};
use cairo_lang_runner::{build_hints_dict, CairoHintProcessor};
use cairo_vm::cairo_run::{cairo_run_program, CairoRunConfig};
use cairo_vm::types::program::Program;
use cairo_vm::types::relocatable::MaybeRelocatable;
use cairo_vm::Felt252;
use camino::Utf8PathBuf;
use clap::Parser;
use scarb_metadata::{MetadataCommand, PackageMetadata, ScarbCommand};
use scarb_ui::args::PackagesFilter;
use scarb_ui::{OutputFormat, Ui};
use std::fs::File;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(author, version, about = "Execute a Scarb-built Cairo package.")]
struct Args {
    /// Name of the package.
    #[command(flatten)]
    packages_filter: PackagesFilter,

    /// Do not rebuild the package (use existing build artifacts).
    #[arg(long, default_value_t = false)]
    no_build: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let ui = Ui::new(Default::default(), OutputFormat::Text);

    // Retrieve workspace metadata.
    let metadata = match MetadataCommand::new().inherit_stderr().exec() {
        Ok(metadata) => metadata,
        Err(error) => {
            eprintln!("Failed to retrieve metadata: {error:?}");
            return ExitCode::FAILURE;
        }
    };

    // Load the targeted package metadata.
    let package = match args.packages_filter.match_one(&metadata) {
        Ok(package) => package,
        Err(error) => {
            eprintln!("Failed to find a matching package in the workspace: {error:?}");
            return ExitCode::FAILURE;
        }
    };

    // Build the package if `--no-build` is not passed.
    if !args.no_build {
        ui.print(format!("Building package: {}", package.name));
        if let Err(error) = ScarbCommand::new().arg("build").run() {
            eprintln!("Failed to build package: {error:?}");
            return ExitCode::FAILURE;
        }
    }

    // Locate the executable file.
    let executable_path = find_executable(&package).unwrap();

    ui.print(format!("Found executable: {}", executable_path));

    // Run the executable with any provided arguments.
    run_executable(&executable_path)
}

/// Find the executable for the given package.
fn find_executable(package: &PackageMetadata) -> Result<Utf8PathBuf> {
    let build_dir =
        Utf8PathBuf::from(std::env::var("SCARB_TARGET_DIR").unwrap_or_else(|_| "target".into()));
    let profile = std::env::var("SCARB_PROFILE").unwrap_or_else(|_| "debug".into()); // Use "debug" by default.
    let executable_name = &package.name; // The executable name often matches the package name.

    let executable_path = build_dir.join(profile).join("bin").join(executable_name);
    ensure!(
        executable_path.exists(),
        format!(
            "Executable `{}` not found. Did you forget to build the package?",
            executable_path
        )
    );
    Ok(executable_path)
}

/// Run the executable.
fn run_executable(executable_path: &Utf8PathBuf) -> ExitCode {
    let file = File::open(executable_path).unwrap();
    let executable: Executable = serde_json::from_reader(file).unwrap();

    // Convert the executable path into a valid string.
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
            .find(|e| matches!(e.kind, EntryPointKind::NonReturning)) // ?? Bootloader 2.9.2 TODO: fixme
            .unwrap();
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
        .unwrap()
    };

    let mut hint_processor = CairoHintProcessor {
        runner: None,
        user_args: vec![vec![]], // is relevant??
        string_to_hint,
        starknet_state: Default::default(),
        run_resources: Default::default(),
        syscalls_used_resources: Default::default(),
        no_temporary_segments: false,
    };

    let cairo_run_config = CairoRunConfig::default();

    // Execute the Cairo program.
    if let Err(_err) = cairo_run_program(&program, &cairo_run_config, &mut hint_processor) {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
