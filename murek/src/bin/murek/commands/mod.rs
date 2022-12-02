#![allow(clippy::module_inception)]

use anyhow::Result;

use murek::core::Config;

use crate::args::Command;

pub mod add;
pub mod build;
pub mod clean;
pub mod commands;
pub mod external;
pub mod manifest_path;

pub fn run(command: Command, config: &mut Config) -> Result<()> {
    use Command::*;

    match command {
        // Keep these sorted alphabetically.
        Add => add::run(config),
        Build => build::run(config),
        Clean => clean::run(config),
        External(args) => external::run(args, config),
        Commands => commands::run(config),
        ManifestPath => manifest_path::run(config),
    }
}
