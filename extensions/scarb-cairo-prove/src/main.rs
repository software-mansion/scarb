use anyhow::{ensure, Context, Result};
use camino::Utf8PathBuf;
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
#[derive(Parser, Debug)]
#[clap(version, verbatim_doc_comment)]
struct Args {
    /// Name of the package.
    #[command(flatten)]
    packages_filter: PackagesFilter,

    /// Number of `cairo-execute` output for given package, for which to generate proof.
    #[arg(long)]
    execution: Option<u32>,

    /// The AIR public input path.
    #[arg(long, value_name = "PUBLIC_INPUT_PATH",required_unless_present_any = ["execution"], conflicts_with_all = ["execution"])]
    pub_input: Option<Utf8PathBuf>,

    /// The AIR private input path.
    #[arg(long, value_name = "PRIVATE_INPUT_PATH", required_unless_present_any = ["execution"], conflicts_with_all = ["execution"])]
    priv_input: Option<Utf8PathBuf>,
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
    let (pub_input, priv_input, proof_path) = if let Some(execution_num) = args.execution {
        // Package-based mode
        let metadata = MetadataCommand::new().inherit_stderr().exec()?;
        let package = args
            .packages_filter
            .match_one(&metadata)
            .context("Failed to find a matching package in the workspace")?;

        ui.print(Status::new("Proving", &package.name));

        let scarb_target_dir = Utf8PathBuf::from(env::var("SCARB_TARGET_DIR")?);
        let execution_dir = scarb_target_dir
            .join("scarb-execute")
            .join(&package.name)
            .join(format!("execution{}", execution_num));

        ensure!(
            execution_dir.exists(),
            formatdoc! {r#"
            Execution directory not found: {}
            Make sure to run scarb cairo-execute first
        "#, execution_dir}
        );

        // Get input files from execution directory
        let pub_input = execution_dir.join("air_public_input.json");
        let priv_input = execution_dir.join("air_private_input.json");

        ensure!(
            pub_input.exists() && priv_input.exists(),
            formatdoc! {r#"
                Missing input files in execution directory: {}
                Make sure both air_public_input.json and air_private_input.json exist
            "#, execution_dir}
        );

        // Create proof directory inside execution directory
        let proof_dir = execution_dir.join("proof");
        create_output_dir(proof_dir.as_std_path())?;

        (pub_input, priv_input, proof_dir.join("proof.json"))
    } else {
        // Raw file paths mode
        let pub_input_path = args.pub_input.unwrap();
        let priv_input_path = args.priv_input.unwrap();

        ui.print(Status::new("Proving", "Cairo program"));

        ensure!(pub_input_path.exists(), "Public input file not found");
        ensure!(priv_input_path.exists(), "Private input file not found");

        // Create proof file in current directory
        let proof_path = Utf8PathBuf::from("proof.json");

        (pub_input_path, priv_input_path, proof_path)
    };

    // Generate proof
    let prover_input = adapt_vm_output(pub_input.as_std_path(), priv_input.as_std_path(), false)
        .context("Failed to adapt VM output")?;

    let config = ProverConfig::builder().build();
    let proof = prove_cairo::<Blake2sMerkleChannel>(prover_input, config)
        .context("Failed to generate proof")?;

    // Save proof output
    fs::write(proof_path.as_std_path(), serde_json::to_string(&proof)?)
        .context("Failed to write proof file")?;

    ui.print(Status::new("Proof saved to:", proof_path.as_str()));
    Ok(())
}
