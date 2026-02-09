#![deny(clippy::dbg_macro)]
#![deny(clippy::disallowed_methods)]

use anyhow::{Context, Result, bail, ensure};
use cairo_air::utils::ProofFormat;
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use create_output_dir::create_output_dir;
use indoc::{formatdoc, indoc};
use mimalloc::MiMalloc;
use scarb_extensions_cli::execute::{ExecutionTarget, OutputFormat, ToArgs};
use scarb_extensions_cli::prove::Args;
use scarb_fs_utils::{MANIFEST_FILE_NAME, find_manifest_path};
use scarb_metadata::{Metadata, MetadataCommand, ScarbCommand};
use scarb_ui::Ui;
use scarb_ui::args::{PackagesFilter, ToEnvVars};
use scarb_ui::components::Status;
use std::fs;
use std::process::ExitCode;
use std::{env, io};
use stwo_cairo_adapter::ProverInput;
use stwo_cairo_prover::prover::create_and_serialize_proof;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

const MAX_ITERATION_COUNT: usize = 10000;

fn main() -> ExitCode {
    let args = Args::parse();
    let ui = Ui::new(args.verbose.clone().into(), args.output_format());

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

    let scarb_target_dir = scarb_target_dir_from_env()?;

    let metadata = MetadataCommand::new()
        .envs(args.execute_args.features.clone().to_env_vars())
        .inherit_stderr()
        .exec()?;
    let package = args.packages_filter.match_one(&metadata)?;

    let execution_id = match args.execution_id {
        Some(id) => id,
        None => {
            assert!(args.execute);
            let output_dir = scarb_target_dir.join("execute").join(&package.name);
            create_output_dir(output_dir.as_std_path())?;
            let (_execution_output_dir, execution_id) = incremental_create_output_dir(&output_dir)?;

            let filter = PackagesFilter::generate_for::<Metadata>(vec![package.clone()].iter());
            ensure!(
                args.execute_args
                    .run
                    .target
                    .as_ref()
                    .map(|t| t.is_bootloader())
                    .unwrap_or(true),
                "only bootloader execution can be proven with `scarb prove` command"
            );
            let mut cmd = ScarbCommand::new();
            cmd.arg("execute")
                .args(args.execute_args.to_args())
                .env("SCARB_EXECUTION_ID", execution_id.to_string())
                .env("SCARB_PACKAGES_FILTER", filter.to_env())
                .env("SCARB_UI_VERBOSITY", ui.verbosity().to_string())
                .envs(args.execute_args.features.clone().to_env_vars());
            if args.execute_args.run.target.is_none() {
                cmd.arg(format!("--target={}", ExecutionTarget::Bootloader));
            }
            if args.execute_args.run.output.is_none() {
                cmd.arg(format!("--output={}", OutputFormat::Standard));
            }
            cmd.run().with_context(|| "execution failed")?;

            execution_id
        }
    };
    ui.print(Status::new("Proving", &package.name));

    let (prover_input_path, proof_path) =
        resolve_paths_from_package(&scarb_target_dir, &package.name, execution_id)?;

    let prover_input: ProverInput =
        serde_json::from_str(fs::read_to_string(prover_input_path)?.as_str())?;

    create_and_serialize_proof(
        prover_input,
        false,
        proof_path.as_std_path().to_path_buf(),
        ProofFormat::Json,
        None,
    )?;

    ui.print(Status::new(
        "Saving proof to:",
        &display_path(&scarb_target_dir, &proof_path),
    ));

    Ok(())
}

fn scarb_target_dir_from_env() -> Result<Utf8PathBuf> {
    match env::var("SCARB_TARGET_DIR") {
        Ok(value) => Ok(Utf8PathBuf::from(value)),
        Err(_) => {
            let manifest_path = find_manifest_path(None)?;
            if manifest_path.exists() {
                bail!("`SCARB_TARGET_DIR` env var must be defined")
            } else {
                bail!(
                    "no {MANIFEST_FILE_NAME} found, this command must be run inside a Scarb project"
                )
            }
        }
    }
}

fn resolve_paths_from_package(
    scarb_target_dir: &Utf8PathBuf,
    package_name: &str,
    execution_id: usize,
) -> Result<(Utf8PathBuf, Utf8PathBuf)> {
    let execution_dir = scarb_target_dir
        .join("execute")
        .join(package_name)
        .join(format!("execution{execution_id}",));

    ensure!(
        execution_dir.exists(),
        formatdoc! {r#"
            execution directory not found: {execution_dir}
            help: make sure to run `scarb execute` first
            and then run `scarb prove` with correct execution ID
            "#, }
    );

    let cairo_pie_path = execution_dir.join("cairo_pie.zip");
    ensure!(
        !cairo_pie_path.exists(),
        formatdoc! {r#"
            proving cairo pie output is not supported: {cairo_pie_path}
            help: run `scarb execute --output=standard` first
            and then run `scarb prove` with correct execution ID
            "#, }
    );

    // Get input files from execution directory
    let prover_input_path = execution_dir.join("prover_input.json");
    ensure!(
        prover_input_path.exists(),
        format!("prover input file does not exist at path: {prover_input_path}")
    );

    // Create proof directory under this execution folder
    let proof_dir = execution_dir.join("proof");
    create_output_dir(proof_dir.as_std_path()).context("failed to create proof directory")?;
    let proof_path = proof_dir.join("proof.json");

    Ok((prover_input_path, proof_path))
}

fn display_path(scarb_target_dir: &Utf8Path, output_path: &Utf8Path) -> String {
    match output_path.strip_prefix(scarb_target_dir) {
        Ok(stripped) => Utf8PathBuf::from("target").join(stripped).to_string(),
        Err(_) => output_path.to_string(),
    }
}

fn incremental_create_output_dir(path: &Utf8Path) -> Result<(Utf8PathBuf, usize)> {
    for i in 1..=MAX_ITERATION_COUNT {
        let filepath = path.join(format!("execution{i}"));
        let result = fs::create_dir(&filepath);
        return match result {
            Err(e) => {
                if e.kind() == io::ErrorKind::AlreadyExists {
                    continue;
                }
                Err(e.into())
            }
            Ok(_) => Ok((filepath, i)),
        };
    }
    bail!("failed to create output directory")
}
