use anyhow::{anyhow, ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use create_output_dir::create_output_dir;
use indoc::formatdoc;
use scarb_metadata::{Metadata, MetadataCommand, PackageMetadata, ScarbCommand};
use scarb_ui::args::{PackagesFilter, VerbositySpec};
use scarb_ui::components::Status;
use scarb_ui::{OutputFormat, Ui, UiPrinter};
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
    #[arg(long)]
    execution_id: Option<u32>,

    /// Execute the program before proving.
    #[arg(long, conflicts_with_all = ["execution_id", "pub_input_file", "priv_input_file"])]
    execute: bool,

    #[command(flatten)]
    execute_args: ExecuteArgs,

    #[command(flatten)]
    files: InputFileArgs,

    #[command(flatten)]
    prover: ProverArgs,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
}

#[derive(Parser, Clone, Debug)]
struct ExecuteArgs {
    /// Do not build the package before execution.
    #[arg(long, requires = "execute")]
    no_build: bool,

    /// Arguments to pass to the program during execution.
    #[arg(long, requires = "execute")]
    arguments: Option<String>,

    /// Arguments to the executable function from a file.
    #[arg(long, conflicts_with = "arguments")]
    arguments_file: Option<String>,

    /// Target for execution.
    #[arg(long, requires = "execute")]
    target: Option<String>,
}

#[derive(Parser, Clone, Debug)]
struct InputFileArgs {
    /// AIR public input path.
    #[arg(long, required_unless_present_any = ["execution_id", "execute"], conflicts_with_all = ["execution_id", "execute"])]
    pub_input_file: Option<Utf8PathBuf>,

    /// AIR private input path.
    #[arg(long, required_unless_present_any = ["execution_id", "execute"], conflicts_with_all = ["execution_id", "execute"])]
    priv_input_file: Option<Utf8PathBuf>,
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
    let scarb_target_dir = Utf8PathBuf::from(env::var("SCARB_TARGET_DIR")?);

    ui.warn("soundness of proof is not yet guaranteed by Stwo, use at your own risk");

    let (pub_input_path, priv_input_path, proof_path) =
        if args.execute || args.execution_id.is_some() {
            let metadata = MetadataCommand::new().inherit_stderr().exec()?;
            let package = args.packages_filter.match_one(&metadata)?;

            let execution_id = match args.execution_id {
                Some(execution_id) => execution_id,
                None => run_execute(&args.execute_args, &package, &scarb_target_dir, &ui)?,
            };

            ui.print(Status::new("Proving", &package.name));

            resolve_paths_from_package(&scarb_target_dir, &package.name, execution_id)?
        } else {
            ui.print(Status::new("Proving", "Cairo program"));

            resolve_paths(&args.files)?
        };

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
    execution_id: u32,
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
            and that the execution ID is correct
        "#, execution_dir}
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

fn resolve_paths(files: &InputFileArgs) -> Result<(Utf8PathBuf, Utf8PathBuf, Utf8PathBuf)> {
    let pub_input_path = files.pub_input_file.clone().unwrap();
    let priv_input_path = files.priv_input_file.clone().unwrap();

    ensure!(
        pub_input_path.exists(),
        format!("public input file does not exist at path: {pub_input_path}")
    );
    ensure!(
        priv_input_path.exists(),
        format!("private input file does not exist at path: {priv_input_path}")
    );

    // Create proof file in current directory
    let proof_path = Utf8PathBuf::from("proof.json");

    Ok((pub_input_path, priv_input_path, proof_path))
}

fn run_execute(
    execution_args: &ExecuteArgs,
    package: &PackageMetadata,
    scarb_target_dir: &Utf8PathBuf,
    ui: &Ui,
) -> Result<u32> {
    let package_filter = PackagesFilter::generate_for::<Metadata>(vec![package.clone()].iter());

    let mut cmd = ScarbCommand::new_for_output();
    cmd.arg("execute")
        .env("SCARB_PACKAGES_FILTER", package_filter.to_env())
        .env("SCARB_TARGET_DIR", scarb_target_dir);

    if execution_args.no_build {
        cmd.arg("--no-build");
    }
    if let Some(arguments) = &execution_args.arguments {
        cmd.arg(format!("--arguments={arguments}"));
    }
    if let Some(arguments_file) = &execution_args.arguments_file {
        cmd.arg(format!("--arguments-file={arguments_file}"));
    }
    if let Some(target) = &execution_args.target {
        cmd.arg(format!("--target={target}"));
    }

    let printer = UiPrinter::new(ui);
    let output = cmd.output_and_stream(&printer)?;
    extract_execution_id(&output)
}

fn extract_execution_id(output: &[String]) -> Result<u32> {
    output
        .iter()
        .find_map(|line| {
            line.trim()
                .strip_prefix("Saving output to:")
                .and_then(|output_path| {
                    Utf8PathBuf::from(output_path.trim())
                        .file_name()
                        .and_then(|name| name.trim_start_matches("execution").parse::<u32>().ok())
                })
        })
        .ok_or_else(|| anyhow!("failed to extract execution ID from `scarb execute` output"))
}

fn display_path(scarb_target_dir: &Utf8Path, output_path: &Utf8Path) -> String {
    match output_path.strip_prefix(scarb_target_dir) {
        Ok(stripped) => Utf8PathBuf::from("target").join(stripped).to_string(),
        Err(_) => output_path.to_string(),
    }
}
