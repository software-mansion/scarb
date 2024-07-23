use crate::args::{Command, TestRunner};
use anyhow::Result;
use inquire::Select;
use which::which;

pub fn resolve_command(command: &Command) -> Result<Command> {
    let mut command = command.clone();
    match &mut command {
        Command::Init(args) if args.test_runner.is_none() => {
            let test_runner = ask_for_test_runner()?;
            args.test_runner = Some(test_runner);
        }
        Command::New(args) if args.init.test_runner.is_none() => {
            let test_runner = ask_for_test_runner()?;
            args.init.test_runner = Some(test_runner);
        }
        _ => {}
    }
    Ok(command)
}

fn ask_for_test_runner() -> Result<TestRunner> {
    let options = if which("snforge").is_ok() {
        vec!["Starknet Foundry (default)", "Cairo Native runner"]
    } else {
        vec![
            "Cairo Native runner (default)",
            "Starknet Foundry (recommended, requires snforge installed)",
        ]
    };

    let answer = Select::new("Which test runner do you want to set up?", options).prompt()?;

    if answer.starts_with("Cairo Native") {
        Ok(TestRunner::CairoNativeRunner)
    } else {
        Ok(TestRunner::StarknetFoundry)
    }
}
