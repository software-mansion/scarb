use crate::args::{CompletionsArgs, ScarbArgs};
use anyhow::{Result, anyhow};
use clap::{Arg, Command, CommandFactory};
use clap_complete::{Shell, generate};
use indoc::indoc;
use scarb::core::Config;
use scarb::ops::{ExternalSubcommand, SubcommandDirs, list_external_subcommands};
use scarb_extensions_cli::cairo_language_server as cairo_language_server_args;
use scarb_extensions_cli::cairo_run as cairo_run_args;
use scarb_extensions_cli::cairo_test as cairo_test_args;
use scarb_extensions_cli::doc as doc_args;
use scarb_extensions_cli::execute;
use scarb_extensions_cli::mdbook as mdbook_args;
use scarb_extensions_cli::prove as prove_args;
use scarb_extensions_cli::verify as verify_args;
use std::collections::HashSet;
use std::io;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: CompletionsArgs, config: &Config) -> Result<()> {
    let mut cmd = build_command(config)?;
    let shell = args.shell.or_else(Shell::from_env).ok_or_else(|| {
        anyhow!(indoc! {r#"
            could not automatically determine shell to generate completions for.
            help: specify the shell explicitly: `scarb completions <shell>`.
            For the list of supported shells, run `scarb completions --help`.
        "#})
    })?;
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
    let scarb_exe_dir = &dirs.scarb_exe_dir;
    let external_subcommands = list_external_subcommands(&dirs)?;
    for external_cmd in external_subcommands {
        let is_bundled = external_cmd.path.parent() == Some(scarb_exe_dir.as_path());
        // Generate full completions only for the known bundled subcommands.
        // For the other subcommands, complete only the command name.
        let subcommand = match (is_bundled, external_cmd.name.as_str()) {
            (true, cairo_language_server_args::COMMAND_NAME) => {
                cairo_language_server_args::Args::command()
            }
            (true, cairo_run_args::COMMAND_NAME) => cairo_run_args::Args::command(),
            (true, cairo_test_args::COMMAND_NAME) => cairo_test_args::Args::command(),
            (true, doc_args::COMMAND_NAME) => doc_args::Args::command(),
            (true, execute::COMMAND_NAME) => execute::Args::command(),
            (true, mdbook_args::COMMAND_NAME) => mdbook_args::Args::command(),
            (true, prove_args::COMMAND_NAME) => prove_args::Args::command(),
            (true, verify_args::COMMAND_NAME) => verify_args::Args::command(),
            (true, "test-support") => continue,
            _ => build_placeholder_subcommand(&external_cmd, is_bundled),
        };
        let subcommand = sanitize_subcommand_args(subcommand, &global_args);
        cmd = cmd.subcommand(subcommand);
    }
    Ok(cmd)
}

/// Build a minimal placeholder `Command` for the unknown external subcommand.
fn build_placeholder_subcommand(
    external_subcommand: &ExternalSubcommand,
    is_bundled: bool,
) -> Command {
    let about = if is_bundled {
        format!("Bundled '{}' extension", external_subcommand.name)
    } else {
        format!("External '{}' extension", external_subcommand.name)
    };
    Command::new(&external_subcommand.name)
        .name(&external_subcommand.name)
        .about(about)
        .disable_help_flag(true)
        .disable_version_flag(true)
}

/// Hide unsupported global args from the subcommand completions
/// by overriding them with local args of the same name, and marking those as hidden.
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
