use anyhow::Result;
use clap::{Arg, Command, CommandFactory};
use clap_complete::{Shell as ClapShell, generate};
use scarb::core::Config;
use scarb::ops::{SubcommandDirs, list_external_subcommands};
use std::collections::HashSet;
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
    let global_args: HashSet<String> = ScarbArgs::command()
        .get_arguments()
        .filter(|arg| arg.is_global_set())
        .map(|arg| arg.get_id().to_string())
        .collect();

    let dirs = SubcommandDirs::try_from(config).expect("Failed to get subcommand directories");
    let external_subcommands = list_external_subcommands(&dirs)?;
    for external_cmd in external_subcommands {
        // Generate full completions only for the bundled subcommands
        let subcommand = if external_cmd.is_bundled {
            match external_cmd.name.as_str() {
                cairo_language_server_args::COMMAND_NAME => {
                    cairo_language_server_args::Args::command()
                }
                cairo_run_args::COMMAND_NAME => cairo_run_args::Args::command(),
                cairo_test_args::COMMAND_NAME => cairo_test_args::Args::command(),
                doc_args::COMMAND_NAME => doc_args::Args::command(),
                execute::COMMAND_NAME => execute::Args::command(),
                mdbook_args::COMMAND_NAME => mdbook_args::Args::command(),
                prove_args::COMMAND_NAME => prove_args::Args::command(),
                verify_args::COMMAND_NAME => verify_args::Args::command(),
                "test-support" => {
                    continue;
                }
                _ => Command::new(&external_cmd.name)
                    .name(&external_cmd.name)
                    .about(format!("Bundled '{}' extension", external_cmd.name))
                    .disable_help_flag(true)
                    .disable_version_flag(true),
            }
        } else {
            Command::new(&external_cmd.name)
                .name(&external_cmd.name)
                .about(format!("External '{}' extension", external_cmd.name))
                .disable_help_flag(true)
                .disable_version_flag(true)
        };
        let subcommand = sanitize_subcommand_args(subcommand, &global_args);
        cmd = cmd.subcommand(subcommand);
    }
    Ok(cmd)
}

/// Hide unsupported global args from the subcommand completions by overriding them with local args and marking them as hidden.
fn sanitize_subcommand_args(mut cmd: Command, global_args: &HashSet<String>) -> Command {
    let local_args: HashSet<String> = cmd
        .get_arguments()
        .map(|arg| arg.get_id().to_string())
        .collect();
    for name in global_args.difference(&local_args) {
        cmd = cmd.arg(Arg::new(name).hide(true));
    }
    cmd
}
