use anyhow::Result;
use clap::Command;
use clap::{CommandFactory, Parser};
use clap_complete::{Shell as ClapShell, generate};
use scarb::EXTERNAL_CMD_PREFIX;
use scarb::args::ScarbArgs;
use scarb::core::config::get_app_exe_path;
use scarb::core::dirs::{get_project_dirs, resolve_path_dirs};
use scarb::ops::list_external_subcommands;
use scarb_cairo_run::args as cairo_run_args;
use scarb_cairo_test::args as cairo_test_args;
use scarb_doc::args as doc_args;
use scarb_execute::args as execute_args;
use scarb_mdbook::args as mdbook_args;
use scarb_prove::args as prove_args;
use scarb_ui::{OutputFormat, Ui, Verbosity};
use scarb_verify::args as verify_args;
use std::path::PathBuf;
use std::process::ExitCode;
use std::{env, io};

/// Shells supported for completions generation.
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum Shell {
    Bash,
    Fish,
    Elvish,
    PowerShell,
    Zsh,
}

impl From<Shell> for ClapShell {
    fn from(s: Shell) -> Self {
        match s {
            Shell::Bash => ClapShell::Bash,
            Shell::Elvish => ClapShell::Elvish,
            Shell::Fish => ClapShell::Fish,
            Shell::PowerShell => ClapShell::PowerShell,
            Shell::Zsh => ClapShell::Zsh,
        }
    }
}

#[derive(Parser, Clone, Debug)]
#[clap(version, about = "Generate shell completions for scarb")]
struct Args {
    #[arg(value_enum)]
    shell: Shell,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let ui = Ui::new(Verbosity::Normal, OutputFormat::Text);

    match main_inner(args, ui.clone()) {
        Ok(_execution_id) => ExitCode::SUCCESS,
        Err(error) => {
            ui.error(format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}

fn main_inner(args: Args, _ui: Ui) -> Result<()> {
    let pd = get_project_dirs()?;
    let path_dirs = resolve_path_dirs(None, &pd);
    let external_subcommands = list_external_subcommands(&path_dirs)?;
    generate_completions(args.shell, external_subcommands, &path_dirs)?;
    Ok(())
}

fn generate_completions(
    shell: Shell,
    external_subcommands: Vec<PathBuf>,
    path_dirs: &[PathBuf],
) -> Result<()> {
    let mut cmd = ScarbArgs::command();

    let scarb_exe = get_app_exe_path(path_dirs).expect("Failed to get scarb executable path");
    let scarb_dir = scarb_exe
        .parent()
        .expect("Scarb binary path should always have parent directory.");

    for path in external_subcommands {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| {
                s.trim_start_matches(EXTERNAL_CMD_PREFIX)
                    .trim_end_matches(env::consts::EXE_SUFFIX)
                    .to_owned()
            })
            .expect("could not resolve subcommand name");

        // Generate completions only for the bundled subcommands
        let subcommand = if path.parent() == Some(scarb_dir) {
            match name.as_str() {
                "cairo-language-server" => {
                    Command::new("cairo-language-server").about("Start Cairo Language Server")
                }
                "cairo-run" => cairo_run_args::Args::command().name("cairo-run"),
                "cairo-test" => cairo_test_args::Args::command().name("cairo-test"),
                "doc" => doc_args::Args::command().name("doc"),
                "execute" => execute_args::Args::command().name("execute"),
                "mdbook" => mdbook_args::Args::command().name("mdbook"),
                "prove" => prove_args::Args::command().name("prove"),
                "verify" => verify_args::Args::command().name("verify"),
                _ => Command::new(&name)
                    .name(&name)
                    .about(format!("Bundled '{name}' extension")),
            }
        } else {
            Command::new(&name)
                .name(&name)
                .about(format!("External '{name}' extension"))
        };
        cmd = cmd.subcommand(subcommand);
    }

    let clap_shell: ClapShell = shell.into();
    generate(clap_shell, &mut cmd, "scarb", &mut io::stdout());
    Ok(())
}
