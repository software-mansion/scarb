use anyhow::Result;
use clap::{Command, CommandFactory};
use clap_complete::{Shell as ClapShell, generate};
use scarb::args::{CompletionsArgs, ScarbArgs};
use scarb::core::Config;
use scarb::ops::{SubcommandDirs, list_external_subcommands};
use std::io;

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

    let dirs = SubcommandDirs::try_from(config).expect("Failed to get subcommand directories");
    let external_subcommands = list_external_subcommands(&dirs)?;
    for external_cmd in external_subcommands {
        // Generate full completions only for the bundled subcommands
        let subcommand = if external_cmd.is_bundled {
            match external_cmd.name.as_str() {
                "cairo-language-server" => Some(
                    Command::new("cairo-language-server").about("Start the Cairo Language Server"),
                ),
                "cairo-run" => Some(cairo_run_args::Args::command().name("cairo-run")),
                "cairo-test" => Some(cairo_test_args::Args::command().name("cairo-test")),
                "doc" => Some(doc_args::Args::command().name("doc")),
                "execute" => Some(execute::Args::command().name("execute")),
                "mdbook" => Some(mdbook_args::Args::command().name("mdbook")),
                "prove" => Some(prove_args::Args::command().name("prove")),
                "verify" => Some(verify_args::Args::command().name("verify")),
                "test-support" => None,
                _ => Some(
                    Command::new(&external_cmd.name)
                        .name(&external_cmd.name)
                        .about(format!("Bundled '{}' extension", external_cmd.name)),
                ),
            }
        } else {
            Some(
                Command::new(&external_cmd.name)
                    .name(&external_cmd.name)
                    .about(format!("External '{}' extension", external_cmd.name)),
            )
        };
        if let Some(subcommand) = subcommand {
            cmd = cmd.subcommand(subcommand);
        }
    }
    Ok(cmd)
}
