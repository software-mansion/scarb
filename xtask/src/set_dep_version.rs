use crate::set_scarb_version;
use anyhow::Result;
use clap::{Parser, ValueEnum};
use semver::Version;
use std::mem;
use std::path::PathBuf;
use toml_edit::{DocumentMut, InlineTable, Value};
use xshell::{cmd, Shell};

/// Update toolchain crates properly.
#[derive(Parser)]
pub struct Args {
    /// Name of toolchain dependency (group) to update.
    dep: DepName,

    #[command(flatten)]
    spec: Spec,

    /// Do not edit any files, just inform what would be done.
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[derive(ValueEnum, Copy, Clone, Debug)]
enum DepName {
    Cairo,
    #[value(name = "cairols")]
    CairoLS,
}

#[derive(clap::Args, Clone)]
#[group(required = true, multiple = true)]
struct Spec {
    /// Source the dependency from crates.io and use a specific version.
    version: Option<Version>,

    /// Source the dependency from the GitHub repository and use a specific commit/ref.
    #[arg(short, long, conflicts_with = "branch")]
    rev: Option<String>,

    /// Source the dependency from the GitHub repository and use a specific branch.
    #[arg(short, long)]
    branch: Option<String>,

    /// Source the dependency from a local filesystem.
    ///
    /// This is useful for local development, but avoid commiting this to the repository.
    #[arg(short, long, conflicts_with_all = ["rev", "branch"])]
    path: Option<PathBuf>,
}

pub fn main(args: Args) -> Result<()> {
    let sh = Shell::new()?;

    let mut cargo_toml = sh.read_file("Cargo.toml")?.parse::<DocumentMut>()?;

    edit_dependencies(&mut cargo_toml, &args);
    edit_patch(&mut cargo_toml, &args);

    if !args.dry_run {
        sh.write_file("Cargo.toml", cargo_toml.to_string())?;

        cmd!(sh, "cargo fetch").run()?;

        purge_unused_patches(&mut cargo_toml)?;
        sh.write_file("Cargo.toml", cargo_toml.to_string())?;

        eprintln!("$ cargo xtask set-scarb-version");
        set_scarb_version::main(Default::default())?;
    }

    Ok(())
}

fn edit_dependencies(cargo_toml: &mut DocumentMut, args: &Args) {
    let deps = cargo_toml["workspace"]["dependencies"]
        .as_table_mut()
        .unwrap();

    for (_, dep) in deps.iter_mut().filter(|(key, _)| args.dep.owns(key)) {
        let dep = dep.as_value_mut().unwrap();

        // Always use crates.io requirements so that we can reliably patch them with the
        // `[patch.crates-io]` table.
        let mut new_dep = InlineTable::from_iter([(
            "version",
            match &args.spec.version {
                Some(version) => Value::from(version.to_string()),
                None => Value::from("*"),
            },
        )]);

        copy_dependency_features(&mut new_dep, dep);

        *dep = new_dep.into();
        simplify_dependency_table(dep)
    }

    deps.fmt();
    deps.sort_values();

    eprintln!("[workspace.dependencies]");
    for (key, dep) in deps.iter().filter(|(key, _)| args.dep.owns(key)) {
        eprintln!("{key} = {dep}");
    }
}

fn edit_patch(cargo_toml: &mut DocumentMut, args: &Args) {
    let patch = cargo_toml["patch"].as_table_mut().unwrap()["crates-io"]
        .as_table_mut()
        .unwrap();

    // Clear any existing entries for this dependency.
    for crate_name in args.dep.crates() {
        patch.remove(crate_name);
    }

    // Leave this section as-if if we are requested to just use a specific version.
    if args.spec.rev.is_some() || args.spec.branch.is_some() || args.spec.path.is_some() {
        // Patch all Cairo crates that exist, even if this project does not directly depend on them,
        // to avoid any duplicates in transient dependencies.
        for &dep_name in args.dep.crates() {
            let mut dep = InlineTable::new();

            // Add a Git branch or revision reference if requested.
            if args.spec.rev.is_some() || args.spec.branch.is_some() {
                dep.insert("git", args.dep.repo().into());
            }

            if let Some(branch) = &args.spec.branch {
                dep.insert("branch", branch.as_str().into());
            }

            if let Some(rev) = &args.spec.rev {
                dep.insert("rev", rev.as_str().into());
            }

            // Add local path reference if requested.
            // For local path sources, Cargo is not looking for crates recursively therefore, we
            // need to manually provide full paths to Cairo workspace member crates.
            if let Some(path) = &args.spec.path {
                dep.insert(
                    "path",
                    path.join("crates")
                        .join(dep_name)
                        .to_string_lossy()
                        .into_owned()
                        .into(),
                );
            }

            patch.insert(dep_name, dep.into());
        }
    }

    patch.fmt();
    patch.sort_values();

    eprintln!("[patch.crates-io]");
    for (key, dep) in patch.iter() {
        eprintln!("{key} = {dep}");
    }
}

impl DepName {
    fn crates(&self) -> &'static [&'static str] {
        match self {
            DepName::Cairo => {
                // List of library crates published from the starkware-libs/cairo repository.
                // One can get this list from the `scripts/release_crates.sh` script in that repo.
                // Keep this list sorted for better commit diffs.
                &[
                    "cairo-lang-casm",
                    "cairo-lang-compiler",
                    "cairo-lang-debug",
                    "cairo-lang-defs",
                    "cairo-lang-diagnostics",
                    "cairo-lang-doc",
                    "cairo-lang-eq-solver",
                    "cairo-lang-executable",
                    "cairo-lang-filesystem",
                    "cairo-lang-formatter",
                    "cairo-lang-lowering",
                    "cairo-lang-parser",
                    "cairo-lang-plugins",
                    "cairo-lang-proc-macros",
                    "cairo-lang-project",
                    "cairo-lang-runnable-utils",
                    "cairo-lang-runner",
                    "cairo-lang-semantic",
                    "cairo-lang-sierra",
                    "cairo-lang-sierra-ap-change",
                    "cairo-lang-sierra-gas",
                    "cairo-lang-sierra-generator",
                    "cairo-lang-sierra-to-casm",
                    "cairo-lang-sierra-type-size",
                    "cairo-lang-starknet",
                    "cairo-lang-starknet-classes",
                    "cairo-lang-syntax",
                    "cairo-lang-syntax-codegen",
                    "cairo-lang-test-plugin",
                    "cairo-lang-test-runner",
                    "cairo-lang-test-utils",
                    "cairo-lang-utils",
                ]
            }
            DepName::CairoLS => &["cairo-language-server"],
        }
    }

    fn owns(&self, crate_name: &str) -> bool {
        self.crates().contains(&crate_name)
    }

    fn repo(&self) -> &'static str {
        match self {
            DepName::Cairo => "https://github.com/starkware-libs/cairo",
            DepName::CairoLS => "https://github.com/software-mansion/cairols",
        }
    }
}

/// Copies features from source dependency spec to new dependency table, if exists.
fn copy_dependency_features(dest: &mut InlineTable, src: &Value) {
    if let Some(dep) = src.as_inline_table() {
        if let Some(features) = dep.get("features") {
            dest.insert("features", features.clone());
        }
    }
}

/// Simplifies a `{ version = "V" }` dependency spec to shorthand `"V"` if possible.
fn simplify_dependency_table(dep: &mut Value) {
    *dep = match mem::replace(dep, false.into()) {
        Value::InlineTable(mut table) => {
            if table.len() == 1 {
                table.remove("version").unwrap_or_else(|| table.into())
            } else {
                table.into()
            }
        }

        dep => dep,
    }
}

/// Remove any unused patches from the `[patch.crates-io]` table.
///
/// We are adding patch entries for **all** Cairo crates existing, and some may end up being unused.
/// Cargo is emitting warnings about unused patches and keeps a record of them in the `Cargo.lock`.
/// The goal of this function is to resolve these warnings.
fn purge_unused_patches(cargo_toml: &mut DocumentMut) -> Result<()> {
    let sh = Shell::new()?;
    let cargo_lock = sh.read_file("Cargo.lock")?.parse::<DocumentMut>()?;

    if let Some(unused_patches) = find_unused_patches(&cargo_lock) {
        let patch = cargo_toml["patch"].as_table_mut().unwrap()["crates-io"]
            .as_table_mut()
            .unwrap();

        // Remove any patches that are not for Cairo crates.
        patch.retain(|key, _| !unused_patches.contains(&key.to_owned()));
    }

    Ok(())
}

fn find_unused_patches(cargo_lock: &DocumentMut) -> Option<Vec<String>> {
    Some(
        cargo_lock
            .get("patch")?
            .get("unused")?
            .as_array_of_tables()?
            .iter()
            .flat_map(|table| Some(table.get("name")?.as_str()?.to_owned()))
            .collect(),
    )
}
