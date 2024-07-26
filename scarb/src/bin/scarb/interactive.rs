use std::io::{self, IsTerminal};

use crate::args::TestRunner;
use anyhow::{ensure, Result};
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use which::which;

pub fn ask_for_test_runner() -> Result<TestRunner> {
    ensure!(
        io::stdout().is_terminal(),
        "You are not running in terminal. Please provide the --test-runner flag."
    );

    let options = if which("snforge").is_ok() {
        vec!["Starknet Foundry (default)", "Cairo Test"]
    } else {
        vec![
            "Cairo Test (default)",
            "Starknet Foundry (recommended, requires snforge installed: https://github.com/foundry-rs/starknet-foundry)",
        ]
    };

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Which test runner do you want to set up?")
        .items(&options)
        .default(0)
        .interact()?;

    if options[selection].starts_with("Cairo Test") {
        Ok(TestRunner::CairoTest)
    } else {
        Ok(TestRunner::StarknetFoundry)
    }
}
