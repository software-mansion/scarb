use anyhow::Result;
use clap::{Command, CommandFactory};
use clap_complete::{Shell as ClapShell, generate};
use scarb::args::ScarbArgs;
use scarb::ops::{SubcommandDirs, list_external_subcommands};
use scarb_cairo_run::args as cairo_run_args;
use scarb_cairo_test::args as cairo_test_args;
use scarb_doc::args as doc_args;
use scarb_execute::args as execute_args;
use scarb_mdbook::args as mdbook_args;
use scarb_prove::args as prove_args;
use scarb_verify::args as verify_args;
use std::io;

pub mod args;
use args::Args;

pub fn main_inner(args: Args) -> Result<()> {
    let mut cmd = build_command()?;

    let clap_shell: ClapShell = args.shell.into();
    generate(clap_shell, &mut cmd, "scarb", &mut io::stdout());
    Ok(())
}

fn build_command() -> Result<Command> {
    let mut cmd = ScarbArgs::command();

    let dirs = SubcommandDirs::new(None)?;
    let external_subcommands = list_external_subcommands(&dirs)?;
    for external_cmd in external_subcommands {
        // Generate completions only for the bundled subcommands
        let subcommand = if external_cmd.is_bundled {
            match external_cmd.name.as_str() {
                "cairo-language-server" => Some(
                    Command::new("cairo-language-server").about("Start the Cairo Language Server"),
                ),
                "cairo-run" => Some(cairo_run_args::Args::command().name("cairo-run")),
                "cairo-test" => Some(cairo_test_args::Args::command().name("cairo-test")),
                "completions" => Some(Args::command().name("completions")),
                "doc" => Some(doc_args::Args::command().name("doc")),
                "execute" => Some(execute_args::Args::command().name("execute")),
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
