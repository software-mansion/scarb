use crate::CARGO_MANIFEST_FILE_NAME;
use crate::compiler::ProcMacroCompilationUnit;
use crate::compiler::plugin::proc_macro::shared_library_provider::ProcMacroPathsProvider;
use crate::core::{Config, Package, Workspace};
use crate::internal::fsx;
use crate::ops::PackageOpts;
use crate::process::exec_piping;
use anyhow::{Context, Result, anyhow};
use camino::Utf8PathBuf;
use cargo_metadata::MetadataCommand;
use flate2::read::GzDecoder;
use ra_ap_toolchain::Tool;
use scarb_ui::{Message, OutputFormat};
use serde::{Serialize, Serializer};
use serde_json::value::RawValue;
use std::fmt::Display;
use std::fs;
use std::io::{Seek, SeekFrom};
use std::ops::Deref;
use std::process::Command;
use tar::Archive;
use tracing::trace_span;

pub const PROC_MACRO_BUILD_PROFILE: &str = "release";

pub fn compile_unit(unit: ProcMacroCompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let package = unit.components.first().unwrap().package.clone();
    run_cargo(CargoAction::Build, &package, ws)
}

pub fn check_unit(unit: ProcMacroCompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let package = unit.components.first().unwrap().package.clone();
    run_cargo(CargoAction::Check, &package, ws)
}

pub fn get_cargo_package_name(package: &Package) -> Result<String> {
    let cargo_toml_path = package.root().join(CARGO_MANIFEST_FILE_NAME);

    let cargo_toml: toml::Value = toml::from_str(
        &fs::read_to_string(cargo_toml_path).context("could not read `Cargo.toml`")?,
    )
    .context("could not convert `Cargo.toml` to toml")?;

    let package_section = cargo_toml
        .get("package")
        .ok_or_else(|| anyhow!("could not get `package` section from Cargo.toml"))?;

    let package_name = package_section
        .get("name")
        .ok_or_else(|| anyhow!("could not get `name` field from Cargo.toml"))?
        .as_str()
        .ok_or_else(|| anyhow!("could not convert package name to string"))?;

    Ok(package_name.to_string())
}

fn get_cargo_package_version(package: &Package) -> Result<String> {
    let span = trace_span!("cargo_metadata_exec");
    let metadata = {
        let _g = span.enter();
        MetadataCommand::new()
            .cargo_path(Tool::Cargo.path())
            .current_dir(package.root())
            .exec()
            .context("could not get Cargo metadata")?
    };

    let cargo_package_name = get_cargo_package_name(package)?;

    let package = metadata
        .packages
        .iter()
        .find(|pkg| pkg.name.as_str() == cargo_package_name)
        .ok_or_else(|| anyhow!("could not get `{cargo_package_name}` package from metadata"))?;

    Ok(package.version.to_string())
}

pub fn get_crate_archive_basename(package: &Package) -> Result<String> {
    let package_name = get_cargo_package_name(package)?;
    let package_version = get_cargo_package_version(package)?;

    Ok(format!("{package_name}-{package_version}"))
}

pub fn unpack_crate(package: &Package, config: &Config) -> Result<()> {
    let archive_basename = get_crate_archive_basename(package)?;
    let archive_name = format!("{archive_basename}.crate");

    let tar = package
        .target_path(config)
        .into_child("package")
        .open_ro_exclusive(&archive_name, &archive_name, config)?;

    // The following implementation has been copied from the `Cargo` codebase with slight modifications only.
    // The original implementation can be found here:
    // https://github.com/rust-lang/cargo/blob/a4600184b8d6619ed0b5a0a19946dbbe97e1d739/src/cargo/ops/cargo_package.rs#L1110

    tar.deref().seek(SeekFrom::Start(0))?;
    let f = GzDecoder::new(tar.deref());
    let dst = tar.parent().unwrap().join(&archive_basename);
    if dst.exists() {
        fsx::remove_dir_all(&dst)?;
    }
    let mut archive = Archive::new(f);
    archive.set_preserve_mtime(false); // Don't set modified time to avoid filesystem errors
    archive.unpack(dst.parent().unwrap())?;

    Ok(())
}

pub fn fetch_crate(package: &Package, ws: &Workspace<'_>) -> Result<()> {
    run_cargo(CargoAction::Fetch, package, ws)
}

pub fn package_crate(package: &Package, opts: &PackageOpts, ws: &Workspace<'_>) -> Result<()> {
    run_cargo(CargoAction::Package(opts.clone()), package, ws)
}

fn run_cargo(action: CargoAction, package: &Package, ws: &Workspace<'_>) -> Result<()> {
    let cmd = CargoCommand {
        action,
        current_dir: package.root().to_path_buf(),
        output_format: ws.config().ui().output_format(),
        target_dir: package
            .target_path(ws.config())
            .path_unchecked()
            .to_path_buf(),
        config: ws.config(),
    };
    let span = trace_span!("proc_macro");
    {
        let _guard = span.enter();
        exec(&mut cmd.into(), ws.config())?;
    }
    Ok(())
}

#[derive(Clone)]
enum CargoAction {
    Build,
    Check,
    Fetch,
    Package(PackageOpts),
}

struct CargoCommand<'c> {
    current_dir: Utf8PathBuf,
    target_dir: Utf8PathBuf,
    output_format: OutputFormat,
    action: CargoAction,
    config: &'c Config,
}

enum CargoOutputFormat {
    Human,
    Json,
}

impl Display for CargoOutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CargoOutputFormat::Human => write!(f, "human"),
            CargoOutputFormat::Json => write!(f, "json"),
        }
    }
}

impl From<OutputFormat> for CargoOutputFormat {
    fn from(format: OutputFormat) -> Self {
        match format {
            OutputFormat::Text => CargoOutputFormat::Human,
            OutputFormat::Json => CargoOutputFormat::Json,
        }
    }
}

impl<'c> From<CargoCommand<'c>> for Command {
    fn from(args: CargoCommand<'c>) -> Self {
        let mut cmd = Command::new(Tool::Cargo.path());
        cmd.current_dir(args.current_dir);
        match args.action {
            CargoAction::Fetch => cmd.arg("fetch"),
            CargoAction::Build => cmd.arg("build"),
            CargoAction::Check => cmd.arg("check"),
            CargoAction::Package(_) => cmd.arg("package"),
        };
        if args.config.offline() {
            cmd.arg("--offline");
        }
        match args.action {
            CargoAction::Fetch => (),
            CargoAction::Package(ref opts) => {
                cmd.arg("--target-dir");
                cmd.arg(args.target_dir);
                cmd.arg("--no-verify");
                if !opts.check_metadata {
                    cmd.arg("--no-metadata");
                }
                if opts.allow_dirty {
                    cmd.arg("--allow-dirty");
                }
            }
            _ => {
                cmd.arg("--release");
                cmd.arg("--message-format");
                let output_format: CargoOutputFormat = args.output_format.into();
                cmd.arg(output_format.to_string());
                cmd.arg("--target-dir");
                cmd.arg(args.target_dir);
            }
        }
        cmd
    }
}

fn exec(cmd: &mut Command, config: &Config) -> Result<()> {
    exec_piping(
        cmd,
        config,
        |line: &str| config.ui().print(PipedText::new(line)),
        |line: &str| config.ui().print(PipedText::new(line)),
    )
}

/// This message can be used for piped text from subprocesses.
///
/// It accepts either a string or a JSON string.
/// If the input is a JSON string, it can be serialized as a structured message.
/// Otherwise, the structured message will be skipped.
pub struct PipedText(String);

impl PipedText {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

impl Message for PipedText {
    fn text(self) -> String {
        self.0
    }

    fn structured<S: Serializer>(self, ser: S) -> Result<S::Ok, S::Error> {
        match serde_json::from_str::<&RawValue>(self.0.as_str()) {
            Ok(value) => value.serialize(ser),
            Err(_e) => Self::skip_structured(ser),
        }
    }
}
