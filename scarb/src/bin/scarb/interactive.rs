use std::io::{self, IsTerminal};

use crate::args::TestRunner;
use anyhow::{Result, ensure};
use dialoguer::Select;
use dialoguer::theme::ColorfulTheme;
use indoc::indoc;

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
            help: please provide the --test-runner flag or --no-test
        "}
    );

    let options = vec!["Starknet Foundry (default)", "None"];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Which test runner do you want to set up?")
        .items(&options)
        .default(0)
        .interact()?;

    if options[selection].starts_with("None") {
        Ok(TestRunner::None)
    } else {
        Ok(TestRunner::StarknetFoundry)
    }
}
