use std::env;
use std::fs;

use anyhow::{bail, Context, Result};
use cairo_lang_runner::short_string::as_cairo_short_string;
use cairo_lang_runner::{SierraCasmRunner, StarknetState};
use cairo_lang_sierra::extensions::gas::{
    BuiltinCostWithdrawGasLibfunc, RedepositGasLibfunc, WithdrawGasLibfunc,
};
use cairo_lang_sierra::extensions::NamedLibfunc;

use camino::Utf8PathBuf;
use clap::Parser;

use indoc::formatdoc;
use scarb_metadata::packages_filter::PackagesFilter;
use scarb_metadata::MetadataCommand;

/// Execute the main function of a package.
#[derive(Parser, Clone, Debug)]
#[command(author, version)]
struct Args {
    /// Name of the package.
    #[command(flatten)]
    packages_filter: PackagesFilter,

    /// Maximum amount of gas available to the program.
    #[arg(long)]
    available_gas: Option<usize>,

    /// Print more items in memory.
    #[arg(long, default_value_t = false)]
    print_full_memory: bool,
}

fn main() -> Result<()> {
    let args: Args = Args::parse();
    let metadata = MetadataCommand::new().inherit_stderr().exec()?;

    let package = args.packages_filter.match_one(&metadata)?;
    println!("running {} ...", package.name);

    let filename = format!("{}.sierra", package.name);
    let path = Utf8PathBuf::from(env::var("SCARB_TARGET_DIR")?)
        .join(env::var("SCARB_PROFILE")?)
        .join(filename.clone());

    if !path.exists() {
        bail!(formatdoc! {r#"
            package has not been compiled, file does not exist: {filename}
            help: run `scarb build` to compile the package
        "#});
    }

    let Ok(sierra_program) = cairo_lang_sierra::ProgramParser::new()
        .parse(&fs::read_to_string(path.clone()).with_context(|| format!("failed to read Sierra file: {path}"))?)
        else {
            bail!("failed to parse sierra program: {path}")
    };

    if args.available_gas.is_none()
        && sierra_program.type_declarations.iter().any(|decl| {
            matches!(
                decl.long_id.generic_id.0.as_str(),
                WithdrawGasLibfunc::STR_ID
                    | BuiltinCostWithdrawGasLibfunc::STR_ID
                    | RedepositGasLibfunc::STR_ID
            )
        })
    {
        bail!("Program requires gas counter, please provide `--available-gas` argument.");
    }

    let runner = SierraCasmRunner::new(
        sierra_program,
        if args.available_gas.is_some() {
            Some(Default::default())
        } else {
            None
        },
        Default::default(),
    )?;

    let result = runner.run_function(
        runner.find_function("::main")?,
        &[],
        args.available_gas,
        StarknetState::default(),
    )?;

    match result.value {
        cairo_lang_runner::RunResultValue::Success(values) => {
            println!("Run completed successfully, returning {values:?}")
        }
        cairo_lang_runner::RunResultValue::Panic(values) => {
            print!("Run panicked with [");
            for value in &values {
                match as_cairo_short_string(value) {
                    Some(as_string) => print!("{value} ('{as_string}'), "),
                    None => print!("{value}, "),
                }
            }
            println!("].")
        }
    }

    if let Some(gas) = result.gas_counter {
        println!("Remaining gas: {gas}");
    }

    if args.print_full_memory {
        print!("Full memory: [");
        for cell in &result.memory {
            match cell {
                None => print!("_, "),
                Some(value) => print!("{value}, "),
            }
        }
        println!("]");
    }

    Ok(())
}
