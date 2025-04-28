use anyhow::Result;
use clap::Command;
use clap::{CommandFactory, Parser};
use clap_complete::{Shell as ClapShell, generate};
use scarb::args::ScarbArgs;
use scarb::core::config::get_app_exe_path;
use scarb::core::dirs::{get_project_dirs, resolve_path_dirs};
use scarb::ops::list_external_subcommands;
use scarb_ui::{OutputFormat, Ui, Verbosity};
use std::{env, io};
use std::path::PathBuf;
use std::process::ExitCode;
use scarb::EXTERNAL_CMD_PREFIX;
use scarb_cairo_run::args as cairo_run_args;
use scarb_cairo_test::args as cairo_test_args;
use scarb_doc::args as doc_args;
use scarb_execute::args as execute_args;
use scarb_mdbook::args as mdbook_args;
use scarb_prove::args as prove_args;
use scarb_verify::args as verify_args;

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
        let Some(name) = path.file_name().and_then(|n| n.to_str()).map(|s| {
            s.trim_start_matches(EXTERNAL_CMD_PREFIX)
                .trim_end_matches(env::consts::EXE_SUFFIX)
                .to_owned()
        }) else {
            continue;
        };

        // Provide completions only for the bundled subcommands
        if path.parent() == Some(scarb_dir) {
            match name.as_str() {
                "execute" => {
                    let execute_cmd = execute_args::Args::command().name("execute");
                    cmd = cmd.subcommand(execute_cmd);
                }
                "prove" => {
                    let prove_cmd = prove_args::Args::command().name("prove");
                    cmd = cmd.subcommand(prove_cmd);
                }
                "verify" => {
                    let verify_cmd = verify_args::Args::command().name("verify");
                    cmd = cmd.subcommand(verify_cmd);
                }
                "cairo-run" => {
                    let cairo_run_cmd = cairo_run_args::Args::command().name("cairo-run");
                    cmd = cmd.subcommand(cairo_run_cmd);
                }
                "cairo-test" => {
                    let cairo_test_cmd = cairo_test_args::Args::command().name("cairo-test");
                    cmd = cmd.subcommand(cairo_test_cmd);
                }
                "doc" => {
                    let doc_cmd = doc_args::Args::command().name("doc");
                    cmd = cmd.subcommand(doc_cmd);
                }
                "mdbook" => {
                    let mdbook_cmd = mdbook_args::Args::command().name("mdbook");
                    cmd = cmd.subcommand(mdbook_cmd);
                }
                "cairo-language-server" => {
                    let ls_cmd =
                        Command::new("cairo-language-server").about("Start Cairo Language Server");
                    cmd = cmd.subcommand(ls_cmd);
                }
                _ => {
                    let bundled_cmd = Command::new(&name)
                        .name(&name)
                        .about(format!("Bundled '{name}' extension"));
                    cmd = cmd.subcommand(bundled_cmd);
                }
            }
        } else {
            let external_cmd = Command::new(&name)
                .name(&name)
                .about(format!("External '{name}' extension"));
            cmd = cmd.subcommand(external_cmd);
        }
    }

    let clap_shell: ClapShell = shell.into();
    generate(clap_shell, &mut cmd, "scarb", &mut io::stdout());
    Ok(())
}
