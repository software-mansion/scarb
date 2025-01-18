use anyhow::{anyhow, ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use create_output_dir::create_output_dir;
use indoc::formatdoc;
use scarb_metadata::{Metadata, MetadataCommand, PackageMetadata, ScarbCommand};
use scarb_ui::args::PackagesFilter;
use scarb_ui::components::Status;
use scarb_ui::{OutputFormat, Ui};
use std::env;
use std::fs;
use std::process::{ExitCode, Output};
use stwo_cairo_prover::cairo_air::{prove_cairo, ProverConfig};
use stwo_cairo_prover::input::vm_import::adapt_vm_output;
use stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;

/// Proves `cairo-execute` output using Stwo prover.
#[derive(Parser, Clone, Debug)]
#[clap(version, verbatim_doc_comment)]
struct Args {
    /// Name of the package.
    #[command(flatten)]
    packages_filter: PackagesFilter,

    /// Number of `cairo-execute` *standard* output for given package, for which to generate proof.
    #[arg(long)]
    execution: Option<u32>,

    /// Execute the program before proving.
    #[arg(long, conflicts_with_all = ["execution", "pub_input_file", "priv_input_file"])]
    execute: bool,

    /// Execute the program before proving.
    #[command(flatten)]
    execute_args: ExecuteArgs,

    #[command(flatten)]
    files: InputFileArgs,

    #[command(flatten)]
    prover: ProverArgs,
}

#[derive(Parser, Clone, Debug)]
struct ExecuteArgs {
    /// Do not build the package before execution.
    #[arg(long, requires = "execute")]
    no_build: bool,

    /// Arguments to pass to the program during execution.
    #[arg(long, requires = "execute")]
    arguments: Option<String>,

    /// Target for execution.
    #[arg(long, requires = "execute")]
    target: Option<String>,
}

#[derive(Parser, Clone, Debug)]
struct InputFileArgs {
    /// AIR public input path.
    #[arg(long, required_unless_present_any = ["execution", "execute"], conflicts_with_all = ["execution", "execute"])]
    pub_input_file: Option<Utf8PathBuf>,

    /// AIR private input path.
    #[arg(long, required_unless_present_any = ["execution", "execute"], conflicts_with_all = ["execution", "execute"])]
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
    let ui = Ui::new(Default::default(), OutputFormat::Text);

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

    let (pub_input_path, priv_input_path, proof_path) = if args.execute || args.execution.is_some()
    {
        let metadata = MetadataCommand::new().inherit_stderr().exec()?;
        let package = args.packages_filter.match_one(&metadata)?;

        let execution_num = match args.execution {
            Some(execution_num) => execution_num,
            None => run_cairo_execute(&args.execute_args, &package, &scarb_target_dir)?,
        };

        ui.print(Status::new("Proving", &package.name));

        resolve_paths_from_package(&scarb_target_dir, &package.name, execution_num)?
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
    execution_num: u32,
) -> Result<(Utf8PathBuf, Utf8PathBuf, Utf8PathBuf)> {
    let execution_dir = scarb_target_dir
        .join("scarb-execute")
        .join(package_name)
        .join(format!("execution{}", execution_num));

    ensure!(
        execution_dir.exists(),
        formatdoc! {r#"
            execution directory not found: {}
            help: make sure to run `scarb cairo-execute` first
            and that the execution number is correct
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

fn run_cairo_execute(
    execution_args: &ExecuteArgs,
    package: &PackageMetadata,
    scarb_target_dir: &Utf8PathBuf,
) -> Result<u32> {
    let package_filter = PackagesFilter::generate_for::<Metadata>(vec![package.clone()].iter());

    let mut cmd = ScarbCommand::new_with_output();
    cmd.arg("cairo-execute")
        .env("SCARB_PACKAGES_FILTER", package_filter.to_env())
        .env("SCARB_TARGET_DIR", scarb_target_dir);

    if execution_args.no_build {
        cmd.arg("--no-build");
    }
    if let Some(arguments) = &execution_args.arguments {
        cmd.arg(format!("--arguments={arguments}"));
    }
    if let Some(target) = &execution_args.target {
        cmd.arg(format!("--target={target}"));
    }

    let output = cmd.run_with_output()?;
    extract_execution_num(&output)
}

fn extract_execution_num(output: &Output) -> Result<u32> {
    let stdout = String::from_utf8(output.stdout.clone())
        .context("failed to parse `cairo-execute` output")?;

    stdout
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("Saving output to:")
                // Isolate the last path component (e.g., "execution1"), strip "execution" prefix, and parse the number
                .and_then(|output_path| output_path.trim().split('/').last())
                .and_then(|execution_str| {
                    execution_str
                        .trim_start_matches("execution")
                        .parse::<u32>()
                        .ok()
                })
        })
        .ok_or_else(|| anyhow!("failed to extract execution number from `cairo-execute` output"))
}

fn display_path(scarb_target_dir: &Utf8Path, output_path: &Utf8Path) -> String {
    match output_path.strip_prefix(scarb_target_dir) {
        Ok(stripped) => Utf8PathBuf::from("target").join(stripped).to_string(),
        Err(_) => output_path.to_string(),
    }
}
