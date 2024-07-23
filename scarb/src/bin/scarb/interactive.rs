use crate::args::TestRunner;
use anyhow::Result;
use inquire::Select;
use which::which;

pub fn ask_for_test_runner() -> Result<TestRunner> {
    let options = if which("snforge").is_ok() {
        vec!["Starknet Foundry (default)", "Cairo Test"]
    } else {
        vec![
            "Cairo Test (default)",
            "Starknet Foundry (recommended, requires snforge installed)",
        ]
    };

    let answer = Select::new("Which test runner do you want to set up?", options).prompt()?;

    if answer.starts_with("Cairo Test") {
        Ok(TestRunner::CairoTest)
    } else {
        Ok(TestRunner::StarknetFoundry)
    }
}
