use anyhow::{ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use create_output_dir::create_output_dir;
use indoc::formatdoc;
use scarb_metadata::MetadataCommand;
use scarb_ui::args::PackagesFilter;
use scarb_ui::components::Status;
use scarb_ui::{OutputFormat, Ui};
use std::env;
use std::fs;
use std::process::ExitCode;
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

    #[command(flatten)]
    files: InputFileArgs,

    #[command(flatten)]
    prover: ProverArgs,
}

#[derive(Parser, Clone, Debug)]
struct InputFileArgs {
    /// AIR public input path.
    #[arg(long, required_unless_present_any = ["execution"], conflicts_with_all = ["execution"])]
    pub_input_file: Option<Utf8PathBuf>,

    /// AIR private input path.
    #[arg(long, required_unless_present_any = ["execution"], conflicts_with_all = ["execution"])]
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

    let (pub_input_path, priv_input_path, proof_path) = if let Some(execution_num) = args.execution
    {
        let metadata = MetadataCommand::new().inherit_stderr().exec()?;
        let package = args.packages_filter.match_one(&metadata)?;

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
    .context("Failed to adapt VM output")?;

    let config = ProverConfig::builder()
        .track_relations(args.prover.track_relations)
        .display_components(args.prover.display_components)
        .build();

    let proof = prove_cairo::<Blake2sMerkleChannel>(prover_input, config)
        .context("Failed to generate proof")?;

    // Save proof
    ui.print(Status::new(
        "Saving proof to:",
        &display_path(&scarb_target_dir, &proof_path),
    ));
    fs::write(proof_path.as_std_path(), serde_json::to_string(&proof)?)
        .context("Failed to write proof file")?;

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
            Execution directory not found: {}
            Make sure to run `scarb cairo-execute` first
        "#, execution_dir}
    );

    // Get input files from execution directory
    let pub_input_path = execution_dir.join("air_public_input.json");
    let priv_input_path = execution_dir.join("air_private_input.json");
    ensure!(
        pub_input_path.exists() && priv_input_path.exists(),
        formatdoc! {r#"
            Missing input files in directory: {}
            Make sure air_public_input.json and air_private_input.json exist
        "#, execution_dir}
    );

    // Create proof directory under this execution folder
    let proof_dir = execution_dir.join("proof");
    create_output_dir(proof_dir.as_std_path()).context("Failed to create proof directory")?;
    let proof_path = proof_dir.join("proof.json");

    Ok((pub_input_path, priv_input_path, proof_path))
}

fn resolve_paths(files: &InputFileArgs) -> Result<(Utf8PathBuf, Utf8PathBuf, Utf8PathBuf)> {
    let pub_input_path = files.pub_input_file.clone().unwrap();
    let priv_input_path = files.priv_input_file.clone().unwrap();

    ensure!(
        pub_input_path.exists(),
        "Public input file does not exist at path: {pub_input_path}"
    );
    ensure!(
        priv_input_path.exists(),
        "Private input file does not exist at path: {priv_input_path}"
    );

    // Create proof file in current directory
    let proof_path = Utf8PathBuf::from("proof.json");

    Ok((pub_input_path, priv_input_path, proof_path))
}

fn display_path(scarb_target_dir: &Utf8Path, output_path: &Utf8Path) -> String {
    match output_path.strip_prefix(scarb_target_dir) {
        Ok(stripped) => Utf8PathBuf::from("target").join(stripped).to_string(),
        Err(_) => output_path.to_string(),
    }
}
