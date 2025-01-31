use anyhow::{ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use create_output_dir::create_output_dir;
use indoc::{formatdoc, indoc};
use scarb_execute::args::ExecutionArgs;
use scarb_metadata::MetadataCommand;
use scarb_ui::args::{PackagesFilter, VerbositySpec};
use scarb_ui::components::Status;
use scarb_ui::{OutputFormat, Ui};
use std::env;
use std::fs;
use std::process::ExitCode;
use stwo_cairo_prover::cairo_air::{prove_cairo, ProverConfig};
use stwo_cairo_prover::input::vm_import::adapt_vm_output;
use stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;

/// Proves `scarb execute` output using Stwo prover.
#[derive(Parser, Clone, Debug)]
#[clap(version, verbatim_doc_comment)]
struct Args {
    /// Name of the package.
    #[command(flatten)]
    packages_filter: PackagesFilter,

    /// ID of `scarb execute` *standard* output for given package, for which to generate proof.
    #[arg(
        long,
        conflicts_with_all = [
            "execute",
            "no_build",
            "arguments",
            "arguments_file",
            "output",
            "target",
            "print_program_output"
        ]
    )]
    execution_id: Option<usize>,

    /// Execute the program before proving.
    #[arg(
        long,
        default_value_t = false,
        required_unless_present = "execution_id"
    )]
    execute: bool,

    #[command(flatten)]
    execute_args: ExecutionArgs,

    #[command(flatten)]
    prover: ProverArgs,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
}

#[derive(Parser, Clone, Debug)]
struct ProverArgs {
    /// Track relations during proving.
    #[arg(long, default_value = "false")]
    track_relations: bool,

    /// Display components during proving.
    #[arg(long, default_value = "false")]
    display_components: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let ui = Ui::new(args.verbose.clone().into(), OutputFormat::Text);

    match main_inner(args, ui.clone()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            ui.error(format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}

fn main_inner(args: Args, ui: Ui) -> Result<()> {
    ensure!(
        !cfg!(windows),
        indoc! {r#"
            `scarb prove` is not supported on Windows
            help: use WSL or a Linux/macOS machine instead
            "#
        }
    );

    let scarb_target_dir = Utf8PathBuf::from(env::var("SCARB_TARGET_DIR")?);

    let metadata = MetadataCommand::new().inherit_stderr().exec()?;
    let package = args.packages_filter.match_one(&metadata)?;

    let execution_id = match args.execution_id {
        Some(id) => id,
        None => {
            assert!(args.execute);
            scarb_execute::execute(&package, &args.execute_args, &ui)?
        }
    };
    ui.print(Status::new("Proving", &package.name));
    ui.warn("soundness of proof is not yet guaranteed by Stwo, use at your own risk");

    let (pub_input_path, priv_input_path, proof_path) =
        resolve_paths_from_package(&scarb_target_dir, &package.name, execution_id)?;

    let prover_input = adapt_vm_output(
        pub_input_path.as_std_path(),
        priv_input_path.as_std_path(),
        false,
    )
    .context("failed to adapt VM output")?;

    let config = ProverConfig::builder()
        .track_relations(args.prover.track_relations)
        .display_components(args.prover.display_components)
        .build();

    let proof = prove_cairo::<Blake2sMerkleChannel>(prover_input, config)
        .context("failed to generate proof")?;

    ui.print(Status::new(
        "Saving proof to:",
        &display_path(&scarb_target_dir, &proof_path),
    ));

    fs::write(proof_path.as_std_path(), serde_json::to_string(&proof)?)?;

    Ok(())
}

fn resolve_paths_from_package(
    scarb_target_dir: &Utf8PathBuf,
    package_name: &str,
    execution_id: usize,
) -> Result<(Utf8PathBuf, Utf8PathBuf, Utf8PathBuf)> {
    let execution_dir = scarb_target_dir
        .join("execute")
        .join(package_name)
        .join(format!("execution{}", execution_id));

    ensure!(
        execution_dir.exists(),
        formatdoc! {r#"
            execution directory not found: {}
            help: make sure to run `scarb execute` first
            and then run `scarb prove` with correct execution ID
            "#, execution_dir}
    );

    let cairo_pie_path = execution_dir.join("cairo_pie.zip");
    ensure!(
        !cairo_pie_path.exists(),
        formatdoc! {r#"
            proving cairo pie output is not supported: {}
            help: run `scarb execute --output=standard` first
            and then run `scarb prove` with correct execution ID
            "#, cairo_pie_path}
    );

    // Get input files from execution directory
    let pub_input_path = execution_dir.join("air_public_input.json");
    let priv_input_path = execution_dir.join("air_private_input.json");
    ensure!(
        pub_input_path.exists(),
        format!("public input file does not exist at path: {pub_input_path}")
    );
    ensure!(
        priv_input_path.exists(),
        format!("private input file does not exist at path: {priv_input_path}")
    );

    // Create proof directory under this execution folder
    let proof_dir = execution_dir.join("proof");
    create_output_dir(proof_dir.as_std_path()).context("failed to create proof directory")?;
    let proof_path = proof_dir.join("proof.json");

    Ok((pub_input_path, priv_input_path, proof_path))
}

fn display_path(scarb_target_dir: &Utf8Path, output_path: &Utf8Path) -> String {
    match output_path.strip_prefix(scarb_target_dir) {
        Ok(stripped) => Utf8PathBuf::from("target").join(stripped).to_string(),
        Err(_) => output_path.to_string(),
    }
}
