use std::env;
use std::fs;

use anyhow::{bail, ensure, Context, Result};
use cairo_lang_runner::profiling::ProfilingInfoProcessor;
use cairo_lang_runner::short_string::as_cairo_short_string;
use cairo_lang_runner::{RunResultStarknet, RunResultValue, SierraCasmRunner, StarknetState};
use cairo_lang_sierra::program::VersionedProgram;
use camino::Utf8PathBuf;
use clap::Parser;
use indoc::formatdoc;
use serde::Serializer;

use scarb_metadata::{Metadata, MetadataCommand, ScarbCommand};
use scarb_ui::args::PackagesFilter;
use scarb_ui::components::Status;
use scarb_ui::{Message, OutputFormat, Ui, Verbosity};

mod deserialization;

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

    /// Do not rebuild the package.
    #[arg(long, default_value_t = false)]
    no_build: bool,

    /// Run the profiler alongside the program.
    #[arg(long, default_value_t = false)]
    run_profiler: bool,

    /// Program arguments.
    ///
    /// This should be a JSON array of numbers, decimal bigints or recursive arrays of those. For example, pass `[1]`
    /// to the following function `fn main(a: u64)`, or pass `[1, "2"]` to `fn main(a: u64, b: u64)`,
    /// or `[[1, 2], [3, 4, 5]]` to `fn main(t: (u64, u64), v: Array<u64>)`.
    #[arg(default_value = "[]")]
    arguments: deserialization::Args,
}

fn main() -> Result<()> {
    let args: Args = Args::parse();
    let available_gas = GasLimit::parse(args.available_gas);

    let ui = Ui::new(Verbosity::default(), OutputFormat::Text);

    let metadata = MetadataCommand::new().inherit_stderr().exec()?;

    let package = args.packages_filter.match_one(&metadata)?;

    if !args.no_build {
        let filter = PackagesFilter::generate_for::<Metadata>(vec![package.clone()].iter());
        ScarbCommand::new()
            .arg("build")
            .env("SCARB_PACKAGES_FILTER", filter.to_env())
            .run()?;
    }

    let filename = format!("{}.sierra.json", package.name);
    let path = Utf8PathBuf::from(env::var("SCARB_TARGET_DIR")?)
        .join(env::var("SCARB_PROFILE")?)
        .join(filename.clone());

    ensure!(
        path.exists(),
        formatdoc! {r#"
            package has not been compiled, file does not exist: {filename}
            help: run `scarb build` to compile the package
        "#}
    );

    ui.print(Status::new("Running", &package.name));

    let sierra_program = serde_json::from_str::<VersionedProgram>(
        &fs::read_to_string(path.clone())
            .with_context(|| format!("failed to read Sierra file: {path}"))?,
    )
    .with_context(|| format!("failed to deserialize Sierra program: {path}"))?
    .into_v1()
    .with_context(|| format!("failed to load Sierra program: {path}"))?;

    if available_gas.is_disabled() && sierra_program.program.requires_gas_counter() {
        bail!("program requires gas counter, please provide `--available-gas` argument");
    }

    let runner = SierraCasmRunner::new(
        sierra_program.program.clone(),
        if available_gas.is_disabled() {
            None
        } else {
            Some(Default::default())
        },
        Default::default(),
        args.run_profiler,
    )?;

    let result = runner
        .run_function_with_starknet_context(
            runner.find_function("::main")?,
            &args.arguments,
            available_gas.value(),
            StarknetState::default(),
        )
        .context("failed to run the function")?;

    let profiling_info = if args.run_profiler {
        let profiling_info_processor =
            ProfilingInfoProcessor::new(sierra_program.program, Default::default());
        match &result.profiling_info {
            Some(raw_profiling_info) => Some(ProfilingInfoStatus::Success(
                profiling_info_processor
                    .process(raw_profiling_info)
                    .to_string(),
            )),
            None => Some(ProfilingInfoStatus::NotFound),
        }
    } else {
        None
    };

    ui.print(Summary {
        result,
        print_full_memory: args.print_full_memory,
        gas_defined: available_gas.is_defined(),
        profiling_info,
    });

    Ok(())
}

enum ProfilingInfoStatus {
    Success(String),
    NotFound,
}

struct Summary {
    result: RunResultStarknet,
    print_full_memory: bool,
    gas_defined: bool,
    profiling_info: Option<ProfilingInfoStatus>,
}

impl Message for Summary {
    fn print_text(self)
    where
        Self: Sized,
    {
        match self.result.value {
            RunResultValue::Success(values) => {
                println!("Run completed successfully, returning {values:?}")
            }
            RunResultValue::Panic(values) => {
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

        if self.gas_defined {
            if let Some(gas) = self.result.gas_counter {
                println!("Remaining gas: {gas}");
            }
        }

        if self.print_full_memory {
            print!("Full memory: [");
            for cell in &self.result.memory {
                match cell {
                    None => print!("_, "),
                    Some(value) => print!("{value}, "),
                }
            }
            println!("]");
        }

        if let Some(profiling_info) = self.profiling_info {
            match profiling_info {
                ProfilingInfoStatus::Success(info) => println!("Profiling info:\n{}", info),
                ProfilingInfoStatus::NotFound => println!("Warning: Profiling info not found."),
            }
        }
    }

    fn structured<S: Serializer>(self, _ser: S) -> Result<S::Ok, S::Error>
    where
        Self: Sized,
    {
        todo!("JSON output is not implemented yet for this command")
    }
}

enum GasLimit {
    Disabled,
    Unlimited,
    Limited(usize),
}
impl GasLimit {
    pub fn parse(value: Option<usize>) -> Self {
        match value {
            Some(0) => GasLimit::Disabled,
            Some(value) => GasLimit::Limited(value),
            None => GasLimit::Unlimited,
        }
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self, GasLimit::Disabled)
    }

    pub fn is_defined(&self) -> bool {
        !matches!(self, GasLimit::Unlimited)
    }

    pub fn value(&self) -> Option<usize> {
        match self {
            GasLimit::Disabled => None,
            GasLimit::Limited(value) => Some(*value),
            GasLimit::Unlimited => Some(usize::MAX),
        }
    }
}
