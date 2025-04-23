use clap::{CommandFactory, Parser};
use clap_complete::{Shell as ClapShell, generate};
use scarb::args::ScarbArgs;
use std::io;
use std::process::ExitCode;

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
    let Args { shell } = Args::parse();
    if let Err(err) = generate_completions(shell) {
        eprintln!("Error generating completions: {}", err);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn generate_completions(shell: Shell) -> io::Result<()> {
    let mut cmd = ScarbArgs::command();

    // Integrate extensions
    let execute_cmd = scarb_execute::args::Args::command().name("execute");
    cmd = cmd.subcommand(execute_cmd);

    // TODO: 1. Integrate all other extensions
    // TODO: 2. Dynamically resolve what extensions are available

    let clap_shell: ClapShell = shell.into();
    generate(clap_shell, &mut cmd, "scarb", &mut io::stdout());
    Ok(())
}
