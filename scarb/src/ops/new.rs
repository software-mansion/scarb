use anyhow::{bail, ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use indoc::{formatdoc, indoc};
use itertools::Itertools;

use crate::core::{Config, PackageName};
use crate::internal::fsx;
use crate::{ops, DEFAULT_SOURCE_DIR_NAME, DEFAULT_TARGET_DIR_NAME, MANIFEST_FILE_NAME};

#[derive(Debug)]
pub struct InitOptions {
    pub path: Utf8PathBuf,
    pub name: Option<PackageName>,
}

#[derive(Debug)]
pub struct NewResult {
    pub name: PackageName,
}

pub fn new_package(opts: InitOptions, config: &Config) -> Result<NewResult> {
    ensure!(
        !opts.path.exists(),
        "destination `{}` already exists\nUse `scarb init` to initialize the directory.",
        opts.path
    );

    let name = infer_name(opts.name, &opts.path)?;

    mk(
        MkOpts {
            path: opts.path.clone(),
            name: name.clone(),
        },
        config,
    )
    .with_context(|| format!("failed to create package `{name}` at `{}`", opts.path))?;

    Ok(NewResult { name })
}

pub fn init_package(opts: InitOptions, config: &Config) -> Result<NewResult> {
    ensure!(
        !opts.path.join(MANIFEST_FILE_NAME).exists(),
        "`scarb init` cannot be run on existing Scarb packages"
    );

    let name = infer_name(opts.name, &opts.path)?;

    mk(
        MkOpts {
            path: opts.path,
            name: name.clone(),
        },
        config,
    )
    .with_context(|| format!("failed to create package `{name}`",))?;

    Ok(NewResult { name })
}

fn infer_name(name: Option<PackageName>, path: &Utf8Path) -> Result<PackageName> {
    if let Some(name) = name {
        Ok(name)
    } else {
        let Some(file_name) = path.file_name() else {
            bail!(
                "cannot infer package name from path {:?}\nUse --name to override.",
                path.as_os_str()
            );
        };

        PackageName::try_new(file_name)
    }
}

struct MkOpts {
    path: Utf8PathBuf,
    name: PackageName,
}

fn mk(MkOpts { path, name }: MkOpts, config: &Config) -> Result<()> {
    // Create project directory in case we are called from `new` op.
    fsx::create_dir_all(&path)?;

    write_vcs_ignore(&path, config)?;

    // Create the `Scarb.toml` file.
    let manifest_path = path.join(MANIFEST_FILE_NAME);
    fsx::write(
        &manifest_path,
        formatdoc! {r#"
            [package]
            name = "{name}"
            version = "0.1.0"

            # See more keys and their definitions at https://github.com/software-mansion/scarb/blob/main/scarb/src/core/manifest/toml.rs

            [dependencies]
            # foo = {{ path = "vendor/foo" }}
        "#},
    )?;

    // Create hello world source files (with respective parent directories) if source directory
    // does not exist.
    let source_dir = path.join(DEFAULT_SOURCE_DIR_NAME);
    if !source_dir.exists() {
        fsx::create_dir_all(&source_dir)?;

        fsx::write(
            source_dir.join("lib.cairo"),
            indoc! {r#"
                fn fib(a: felt, b: felt, n: felt) -> felt {
                    match n {
                        0 => a,
                        _ => fib(b, a + b, n - 1),
                    }
                }
            "#},
        )?;
    }

    if let Err(err) = ops::read_workspace(&manifest_path, config) {
        config.ui().warn(formatdoc! {r#"
            compiling this new package may not work due to invalid workspace configuration

            {err:?}
        "#})
    }

    Ok(())
}

/// Write VCS ignore file.
fn write_vcs_ignore(path: &Utf8Path, config: &Config) -> Result<()> {
    let patterns = vec![DEFAULT_TARGET_DIR_NAME];

    let gitignore = path.join(".gitignore");
    if !gitignore.exists() {
        let ignore = patterns.join("\n") + "\n";
        fsx::write(&gitignore, ignore)?;
    } else {
        let lines = patterns
            .into_iter()
            .map(|pat| format!("    {pat}"))
            .join("\n");
        config
            .ui()
            .warn(formatdoc! {r#"
                file `{gitignore}` already exists in this directory, ensure following patterns are ignored:

                {lines}
            "#});
    }

    Ok(())
}
