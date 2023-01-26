#![allow(clippy::module_inception)]

use anyhow::Result;

use scarb::core::Config;

use crate::args::Command;

pub mod add;
pub mod build;
pub mod clean;
pub mod commands;
pub mod external;
pub mod fmt;
pub mod init;
pub mod manifest_path;
pub mod metadata;
pub mod new;

pub fn run(command: Command, config: &mut Config) -> Result<()> {
    use Command::*;

    match command {
        // Keep these sorted alphabetically.
        Add => add::run(config),
        Build => build::run(config),
        Clean => clean::run(config),
        Commands => commands::run(config),
        External(args) => external::run(args, config),
        Fmt(args) => fmt::run(args, config),
        Init(args) => init::run(args, config),
        ManifestPath => manifest_path::run(config),
        Metadata(args) => metadata::run(args, config),
        New(args) => new::run(args, config),
    }
}
