use anyhow::{Context, Result, ensure};
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use create_output_dir::create_output_dir;
use indoc::{formatdoc, indoc};
use scarb_execute::args::ExecutionArgs;
use scarb_metadata::MetadataCommand;
use scarb_ui::args::ToEnvVars;
use scarb_ui::components::Status;
use scarb_ui::{OutputFormat, Ui};
use std::env;
use std::fs;
use std::process::ExitCode;
use stwo_cairo_adapter::vm_import::adapt_vm_output;
use stwo_cairo_prover::cairo_air::prover::{
    ProverConfig, ProverParameters, default_prod_prover_parameters, prove_cairo,
};
use stwo_cairo_prover::stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleChannel;

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

    let metadata = MetadataCommand::new()
        .envs(args.execute_args.features.clone().to_env_vars())
        .inherit_stderr()
        .exec()?;
    let package = args.packages_filter.match_one(&metadata)?;

    let execution_id = match args.execution_id {
        Some(id) => id,
        None => {
            assert!(args.execute);
            scarb_execute::execute(&metadata, &package, &args.execute_args, &ui)?
        }
    };
    ui.print(Status::new("Proving", &package.name));
    ui.warn("soundness of proof is not yet guaranteed by Stwo, use at your own risk");

    let (pub_input_path, priv_input_path, proof_path) =
        resolve_paths_from_package(&scarb_target_dir, &package.name, execution_id)?;

    let prover_input = adapt_vm_output(pub_input_path.as_std_path(), priv_input_path.as_std_path())
        .context("failed to adapt VM output")?;

    let config = ProverConfig {
        display_components: args.prover.display_components,
    };

    let ProverParameters { pcs_config } = default_prod_prover_parameters();
    let proof = prove_cairo::<Blake2sMerkleChannel>(prover_input, config, pcs_config)
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
