use anyhow::Result;
use clap::{Arg, Command, CommandFactory};
use clap_complete::{Shell as ClapShell, generate};
use scarb::core::Config;
use scarb::ops::{SubcommandDirs, list_external_subcommands};
use std::io;

use crate::args::{CompletionsArgs, ScarbArgs};
use scarb_extensions_cli::cairo_language_server as cairo_language_server_args;
use scarb_extensions_cli::cairo_run as cairo_run_args;
use scarb_extensions_cli::cairo_test as cairo_test_args;
use scarb_extensions_cli::doc as doc_args;
use scarb_extensions_cli::execute;
use scarb_extensions_cli::mdbook as mdbook_args;
use scarb_extensions_cli::prove as prove_args;
use scarb_extensions_cli::verify as verify_args;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: CompletionsArgs, config: &Config) -> Result<()> {
    let mut cmd = build_command(config)?;
    let shell: ClapShell = args.shell.into();
    generate(shell, &mut cmd, "scarb", &mut io::stdout());
    Ok(())
}

fn build_command(config: &Config) -> Result<Command> {
    let mut cmd = ScarbArgs::command();
    let global_args = cmd
        .get_arguments()
        .filter(|arg| arg.is_global_set())
        .map(|arg| arg.get_id().to_string())
        .collect::<Vec<_>>();

    let dirs = SubcommandDirs::try_from(config).expect("Failed to get subcommand directories");
    let external_subcommands = list_external_subcommands(&dirs)?;
    for external_cmd in external_subcommands {
        // Generate full completions only for the bundled subcommands
        let subcommand = if external_cmd.is_bundled {
            match external_cmd.name.as_str() {
                cairo_language_server_args::COMMAND_NAME => {
                    Some(cairo_language_server_args::Args::command())
                }
                cairo_run_args::COMMAND_NAME => Some(cairo_run_args::Args::command()),
                cairo_test_args::COMMAND_NAME => Some(cairo_test_args::Args::command()),
                doc_args::COMMAND_NAME => Some(doc_args::Args::command()),
                execute::COMMAND_NAME => Some(execute::Args::command()),
                mdbook_args::COMMAND_NAME => Some(mdbook_args::Args::command()),
                prove_args::COMMAND_NAME => Some(prove_args::Args::command()),
                verify_args::COMMAND_NAME => Some(verify_args::Args::command()),
                "test-support" => None,
                _ => Some(
                    Command::new(&external_cmd.name)
                        .name(&external_cmd.name)
                        .about(format!("Bundled '{}' extension", external_cmd.name))
                        .disable_help_flag(true)
                        .disable_version_flag(true)
                        .args(global_args.iter().map(|name| Arg::new(name).hide(true))),
                ),
            }
        } else {
            Some(
                Command::new(&external_cmd.name)
                    .name(&external_cmd.name)
                    .about(format!("External '{}' extension", external_cmd.name))
                    .disable_help_flag(true)
                    .disable_version_flag(true)
                    .hide_possible_values(true)
                    .args(global_args.iter().map(|name| Arg::new(name).hide(true))),
            )
        };
        if let Some(subcommand) = subcommand {
            cmd = cmd.subcommand(subcommand);
        }
    }
    Ok(cmd)
}
