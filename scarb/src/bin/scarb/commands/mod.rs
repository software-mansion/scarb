#![allow(clippy::module_inception)]

use anyhow::Result;

use scarb::core::Config;

use crate::args::{CacheSubcommand, Command};

pub mod add;
pub mod build;
pub mod cache_clean;
pub mod cache_path;
pub mod check;
pub mod clean;
pub mod commands;
mod expand;
pub mod external;
pub mod fetch;
pub mod fmt;
pub mod init;
pub mod manifest_path;
pub mod metadata;
pub mod new;
pub mod package;
mod proc_macro_server;
pub mod publish;
pub mod remove;
pub mod run;
pub mod test;
mod update;

pub fn run(command: Command, config: &mut Config) -> Result<()> {
    use Command::*;

    match command {
        // Keep these sorted alphabetically.
        Add(args) => add::run(args, config),
        Build(args) => build::run(args, config),
        Expand(args) => expand::run(args, config),
        Cache(CacheSubcommand::Clean) => cache_clean::run(config),
        Cache(CacheSubcommand::Path) => cache_path::run(config),
        Check(args) => check::run(args, config),
        Clean => clean::run(config),
        Commands => commands::run(config),
        External(args) => external::run(args, config),
        Fetch => fetch::run(config),
        Fmt(args) => fmt::run(args, config),
        Init(args) => init::run(args, config),
        ManifestPath => manifest_path::run(config),
        Metadata(args) => metadata::run(args, config),
        New(args) => new::run(args, config),
        Package(args) => package::run(args, config),
        ProcMacroServer => proc_macro_server::run(config),
        Publish(args) => publish::run(args, config),
        Remove(args) => remove::run(args, config),
        Run(args) => run::run(args, config),
        Test(args) => test::run(args, config),
        Update => update::run(config),
    }
}
