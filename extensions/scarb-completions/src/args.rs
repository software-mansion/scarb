use clap::{Parser, ValueEnum};
use clap_complete::Shell as ClapShell;

/// Generate shell completions
#[derive(Parser, Clone, Debug)]
#[clap(version, about = "Generate shell completions for scarb")]
pub struct Args {
    #[arg(value_enum)]
    pub shell: Shell,
}

/// Target shell for completion generation
#[derive(ValueEnum, Clone, Debug)]
pub enum Shell {
    Bash,
    Fish,
    Elvish,
    #[clap(name = "powershell", alias = "pwsh")]
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
