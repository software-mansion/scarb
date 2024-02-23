use crate::compiler::plugin::proc_macro::PROC_MACRO_BUILD_PROFILE;
use crate::compiler::CompilationUnit;
use crate::core::{Config, Package, Workspace};
use crate::flock::Filesystem;
use crate::process::exec_piping;
use anyhow::Result;
use camino::Utf8PathBuf;
use libloading::library_filename;
use std::process::Command;
use tracing::trace_span;

/// This trait is used to define the shared library path for a package.
pub trait SharedLibraryProvider {
    /// Location of Cargo `target` directory.
    fn target_path(&self, config: &Config) -> Filesystem;
    /// Location of the shared library for the package.
    fn shared_lib_path(&self, config: &Config) -> Utf8PathBuf;
}

impl SharedLibraryProvider for Package {
    fn target_path(&self, config: &Config) -> Filesystem {
        let ident = format!("{}-{}", self.id.name, self.id.source_id.ident());
        // Defines the Cargo target directory in cache, as:
        // `/(..)/SCARB_CACHE/plugins/proc_macro/<package_name>-<source_id_ident>/v<version>/target/`
        config
            .dirs()
            .procedural_macros_dir()
            .into_child(ident)
            .into_child(format!("v{}", self.id.version))
            .into_child("target")
    }

    fn shared_lib_path(&self, config: &Config) -> Utf8PathBuf {
        let lib_name = library_filename(self.id.name.to_string());
        let lib_name = lib_name
            .into_string()
            .expect("library name must be valid UTF-8");
        // Defines the shared library path inside the target directory, as:
        // `/(..)/target/release/[lib]<package_name>.[so|dll|dylib]`
        self.target_path(config)
            .into_child(PROC_MACRO_BUILD_PROFILE)
            .path_unchecked()
            .join(lib_name)
    }
}

pub fn compile_unit(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let package = unit.components.first().unwrap().package.clone();
    run_cargo(CargoAction::Build, &package, ws)
}

pub fn check_unit(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let package = unit.components.first().unwrap().package.clone();
    run_cargo(CargoAction::Check, &package, ws)
}

pub fn fetch_package(package: &Package, ws: &Workspace<'_>) -> Result<()> {
    run_cargo(CargoAction::Fetch, package, ws)
}

fn run_cargo(action: CargoAction, package: &Package, ws: &Workspace<'_>) -> Result<()> {
    let cmd = CargoCommand {
        action,
        current_dir: package.root().to_path_buf(),
        target_dir: package
            .target_path(ws.config())
            .path_unchecked()
            .to_path_buf(),
    };
    {
        let _ = trace_span!("proc_macro").enter();
        exec(&mut cmd.into(), ws.config())?;
    }
    Ok(())
}

enum CargoAction {
    Build,
    Check,
    Fetch,
}

struct CargoCommand {
    current_dir: Utf8PathBuf,
    target_dir: Utf8PathBuf,
    action: CargoAction,
}

impl From<CargoCommand> for Command {
    fn from(args: CargoCommand) -> Self {
        let mut cmd = Command::new("cargo");
        cmd.current_dir(args.current_dir);
        match args.action {
            CargoAction::Fetch => cmd.arg("fetch"),
            CargoAction::Build => cmd.arg("build"),
            CargoAction::Check => cmd.arg("check"),
        };
        match args.action {
            CargoAction::Fetch => (),
            _ => {
                cmd.arg("--release");
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
        |line: &str| config.ui().print(line),
        |line: &str| config.ui().print(line),
    )
}
