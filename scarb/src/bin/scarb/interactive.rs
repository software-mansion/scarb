use std::io::{self, IsTerminal};

use crate::args::TestRunner;
use anyhow::{ensure, Result};
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use indoc::indoc;
use which::which;

pub fn get_or_ask_for_test_runner(test_runner: Option<TestRunner>) -> Result<TestRunner> {
    Ok(test_runner)
        .transpose()
        .unwrap_or_else(ask_for_test_runner)
}

fn ask_for_test_runner() -> Result<TestRunner> {
    ensure!(
        io::stdout().is_terminal(),
        indoc! {r"
            you are not running in terminal
            help: please provide the --test-runner flag
        "}
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
