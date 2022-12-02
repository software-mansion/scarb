#![deny(missing_docs)]

//! CLI arguments datastructures.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Cairo's project manager.
#[derive(Parser, Clone, Debug)]
#[command(author, version, about)]
pub struct Args {
    /// Override path to a directory containing a **Murek.toml** file.
    #[arg(long, env = "MUREK_MANIFEST_PATH")]
    pub manifest_path: Option<PathBuf>,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,

    /// Subcommand and its arguments.
    #[command(subcommand)]
    pub command: Command,
}

/// Subcommand and its arguments.
#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    // Keep these sorted alphabetically.
    // External should go last.
    /// Add dependencies to a manifest file.
    Add,
    /// Compile current project.
    Build,
    /// Remove generated artifacts.
    Clean,
    /// List installed commands.
    Commands,
    /// Print path to current **Murek.toml** file to standard output.
    ManifestPath,

    /// External command (`murek-*` executable).
    #[command(external_subcommand)]
    External(Vec<OsString>),
}
