use anyhow::{ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use indoc::formatdoc;
use scarb_metadata::{MetadataCommand, PackageMetadata};
use scarb_ui::args::{PackagesFilter, VerbositySpec};
use scarb_ui::components::Status;
use scarb_ui::{OutputFormat, Ui};
use std::env;
use std::fs;
use std::process::ExitCode;
use stwo_cairo_prover::cairo_air::air::CairoProof;
use stwo_cairo_prover::cairo_air::verify_cairo;
use stwo_cairo_prover::stwo_prover::core::vcs::blake2_merkle::{
    Blake2sMerkleChannel, Blake2sMerkleHasher,
};

/// Verifies `scarb prove` output using Stwo verifier.
#[derive(Parser, Clone, Debug)]
#[clap(version, verbatim_doc_comment)]
struct Args {
    /// Name of the package.
    #[command(flatten)]
    packages_filter: PackagesFilter,

    /// ID of `scarb execute` output for given package, for which proof was generated using `scarb prove`.
    #[arg(long)]
    execution_id: Option<u32>,

    /// Proof file path.
    #[arg(
        long,
        required_unless_present = "execution_id",
        conflicts_with = "execution_id"
    )]
    proof_file: Option<Utf8PathBuf>,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
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

    let metadata = MetadataCommand::new().inherit_stderr().exec()?;
    let package = args.packages_filter.match_one(&metadata)?;

    let proof_path = if let Some(execution_id) = args.execution_id {
        ui.print(Status::new("Verifying", &package.name));
        resolve_proof_path_from_package(&scarb_target_dir, &package, execution_id)?
    } else {
        ui.print(Status::new("Verifying", "proof"));
        args.proof_file.unwrap()
    };

    let proof = load_proof(&proof_path)?;

    verify_cairo::<Blake2sMerkleChannel>(proof).with_context(|| "failed to verify proof")?;

    ui.print(Status::new("Verified", "proof successfully"));

    Ok(())
}

fn load_proof(path: &Utf8Path) -> Result<CairoProof<Blake2sMerkleHasher>> {
    ensure!(
        path.exists(),
        format!("proof file does not exist at path: {path}")
    );

    let proof_contents =
        fs::read_to_string(path).with_context(|| format!("failed to read proof file: {path}"))?;
    let proof = serde_json::from_str(&proof_contents)
        .with_context(|| format!("failed to deserialize proof file: {path}"))?;
    Ok(proof)
}

fn resolve_proof_path_from_package(
    scarb_target_dir: &Utf8Path,
    package: &PackageMetadata,
    execution_id: u32,
) -> Result<Utf8PathBuf> {
    let execution_dir = scarb_target_dir
        .join("execute")
        .join(&package.name)
        .join(format!("execution{execution_id}"));

    ensure!(
        execution_dir.exists(),
        formatdoc! {r#"
            execution directory does not exist at path: {execution_dir}
            help: make sure to run `scarb prove --execute` first
            and that the execution ID is correct
        "#}
    );

    let proof_path = execution_dir.join("proof").join("proof.json");
    ensure!(
        proof_path.exists(),
        formatdoc! {r#"
            proof file does not exist at path: {proof_path}
            help: run `scarb prove --execution-id={execution_id}` first
        "#}
    );

    Ok(proof_path)
}
