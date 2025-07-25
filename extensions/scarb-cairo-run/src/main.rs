#![deny(clippy::dbg_macro)]
#![deny(clippy::disallowed_methods)]

use anyhow::{Context, Result, anyhow, bail, ensure};
use cairo_lang_runner::short_string::as_cairo_short_string;
use cairo_lang_runner::{RunResultStarknet, RunResultValue, SierraCasmRunner, StarknetState};
use cairo_lang_sierra::ids::FunctionId;
use cairo_lang_sierra::program::{Function, ProgramArtifact, VersionedProgram};
use camino::Utf8PathBuf;
use clap::Parser;
use indoc::formatdoc;
use scarb_metadata::{
    CompilationUnitMetadata, Metadata, MetadataCommand, PackageId, PackageMetadata, ScarbCommand,
};
use scarb_ui::args::PackagesFilter;
use scarb_ui::components::Status;
use scarb_ui::{Message, OutputFormat, Ui};
use serde::Serializer;
use std::env;
use std::fs;
use std::process::ExitCode;

use scarb_extensions_cli::cairo_run::{Args, deserialization};

const EXECUTABLE_NAME: &str = "main";
const DEFAULT_MAIN_FUNCTION: &str = "::main";

fn main() -> ExitCode {
    let args: Args = Args::parse();
    let ui = Ui::new(args.verbose.clone().into(), OutputFormat::Text);
    ui.warn("`scarb cairo-run` will be deprecated soon\nhelp: use `scarb execute` instead");
    if let Err(err) = main_inner(&ui, args) {
        ui.anyhow(&err);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn main_inner(ui: &Ui, args: Args) -> Result<()> {
    let metadata = MetadataCommand::new().inherit_stderr().exec()?;

    let package = args.packages_filter.match_one(&metadata)?;

    let available_gas = GasLimit::parse(args.available_gas).with_metadata(&metadata, &package)?;

    let program_args = match args.arguments_file {
        Some(path) => serde_json::from_str::<deserialization::Args>(
            &fs::read_to_string(path.clone())
                .with_context(|| format!("failed to read arguments from file: {path}"))?,
        )?,
        None => args.arguments,
    };

    if !args.no_build {
        let filter = PackagesFilter::generate_for::<Metadata>(vec![package.clone()].iter());
        ScarbCommand::new()
            .arg("build")
            .env("SCARB_PACKAGES_FILTER", filter.to_env())
            .env("SCARB_UI_VERBOSITY", ui.verbosity().to_string())
            .run()?;
    }

    let filename = format!("{}.sierra.json", package.name);
    let path = Utf8PathBuf::from(env::var("SCARB_TARGET_DIR")?)
        .join(env::var("SCARB_PROFILE")?)
        .join(filename.clone());

    ensure!(
        path.exists(),
        formatdoc! {r#"
            package has not been compiled, file does not exist: `{filename}`
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
        None,
    )?;

    let result = runner
        .run_function_with_starknet_context(
            main_function(&runner, &sierra_program, args.function.as_deref())?,
            program_args.into(),
            available_gas.value(),
            StarknetState::default(),
        )
        .with_context(|| "failed to run the function")?;

    ui.print(Summary {
        result,
        print_full_memory: args.print_full_memory,
        gas_defined: available_gas.is_defined(),
        detailed_resources: args.print_resource_usage,
    });

    Ok(())
}

fn main_function<'a>(
    runner: &'a SierraCasmRunner,
    sierra_program: &'a ProgramArtifact,
    name: Option<&str>,
) -> Result<&'a Function> {
    let executables = sierra_program
        .debug_info
        .as_ref()
        .and_then(|di| di.executables.get(EXECUTABLE_NAME))
        .cloned()
        .unwrap_or_default();

    // Prioritize `--function` args. First search among executables, then among all functions.
    if let Some(name) = name {
        let name = format!("::{name}");
        return executables
            .iter()
            .find(|fid| {
                fid.debug_name
                    .as_deref()
                    .map(|debug_name| debug_name.ends_with(&name))
                    .unwrap_or_default()
            })
            .map(|fid| find_function(sierra_program, fid))
            .unwrap_or_else(|| Ok(runner.find_function(&name)?));
    }

    // Then check if executables are unambiguous.
    if executables.len() == 1 {
        return find_function(
            sierra_program,
            executables.first().expect("executables can't be empty"),
        );
    }

    // If executables are ambiguous, bail with error.
    if executables.len() > 1 {
        let names = executables
            .iter()
            .flat_map(|fid| fid.debug_name.clone())
            .map(|name| name.to_string())
            .collect::<Vec<_>>();
        let msg = if names.is_empty() {
            "please only mark a single function as executable or enable debug ids and choose function by name".to_string()
        } else {
            format!(
                "please choose a function to run from the list:\n`{}`",
                names.join("`, `")
            )
        };
        bail!("multiple executable functions found\n{msg}");
    }

    // Finally check default function.
    Ok(runner.find_function(DEFAULT_MAIN_FUNCTION)?)
}

fn find_function<'a>(
    sierra_program: &'a ProgramArtifact,
    fid: &FunctionId,
) -> Result<&'a Function> {
    sierra_program
        .program
        .funcs
        .iter()
        .find(|f| f.id == *fid)
        .ok_or_else(|| anyhow!("function not found"))
}

struct Summary {
    result: RunResultStarknet,
    print_full_memory: bool,
    gas_defined: bool,
    detailed_resources: bool,
}

impl Message for Summary {
    fn print_text(self)
    where
        Self: Sized,
    {
        match self.result.value {
            RunResultValue::Success(values) => {
                let values = values
                    .into_iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>();
                let values = values.join(", ");
                println!("Run completed successfully, returning [{values}]")
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

        if self.detailed_resources {
            let resources = self.result.used_resources.basic_resources;
            let sorted_builtins = sort_by_value(&resources.builtin_instance_counter);
            let sorted_syscalls = sort_by_value(&self.result.used_resources.syscalls);

            println!("Resources:");
            println!("\tsteps: {}", resources.n_steps);
            println!("\tmemory holes: {}", resources.n_memory_holes);
            println!("\tbuiltins: ({})", format_items(&sorted_builtins));
            println!("\tsyscalls: ({})", format_items(&sorted_syscalls));
        }
    }

    fn structured<S: Serializer>(self, _ser: S) -> Result<S::Ok, S::Error>
    where
        Self: Sized,
    {
        todo!("JSON output is not implemented yet for this command")
    }
}

fn sort_by_value<'a, K, V, M>(map: M) -> Vec<(&'a K, &'a V)>
where
    M: IntoIterator<Item = (&'a K, &'a V)>,
    V: Ord,
{
    let mut sorted: Vec<_> = map.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    sorted
}

fn format_items<K, V>(items: &[(K, V)]) -> String
where
    K: std::fmt::Debug,
    V: std::fmt::Display,
{
    items
        .iter()
        .map(|(key, value)| format!("{key:?}: {value}"))
        .collect::<Vec<String>>()
        .join(", ")
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

    /// Disable gas based on the compilation unit compiler config.
    pub fn with_metadata(self, metadata: &Metadata, package: &PackageMetadata) -> Result<Self> {
        let compilation_unit = metadata.package_lib_compilation_unit(package.id.clone());
        let cu_enables_gas = compilation_unit
            .map(|cu| cu.compiler_config.clone())
            .and_then(|c| {
                c.as_object()
                    .and_then(|c| c.get("enable_gas").and_then(|x| x.as_bool()))
            })
            // Defaults to true, meaning gas enabled - relies on cli config then.
            .unwrap_or(true);
        ensure!(
            cu_enables_gas || !self.is_defined(),
            "gas calculation disabled for package `{package_name}`, cannot define custom gas limit",
            package_name = package.name
        );
        if cu_enables_gas {
            // Leave unchanged.
            Ok(self)
        } else {
            // Disable gas based on CU config.
            Ok(GasLimit::Disabled)
        }
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self, GasLimit::Disabled)
    }

    /// Returns true if the gas limit has been defined by the user.
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

trait CompilationUnitProvider {
    /// Return the compilation unit for the package's lib target.
    fn package_lib_compilation_unit(
        &self,
        package_id: PackageId,
    ) -> Option<&CompilationUnitMetadata>;
}

impl CompilationUnitProvider for Metadata {
    fn package_lib_compilation_unit(
        &self,
        package_id: PackageId,
    ) -> Option<&CompilationUnitMetadata> {
        self.compilation_units
            .iter()
            .find(|m| m.package == package_id && m.target.kind == LIB_TARGET_KIND)
    }
}

const LIB_TARGET_KIND: &str = "lib";
