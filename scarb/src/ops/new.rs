use anyhow::{bail, ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use indoc::{formatdoc, indoc};

use crate::core::{Config, PackageName};
use crate::internal::fsx;
use crate::{DEFAULT_SOURCE_DIR_NAME, DEFAULT_TARGET_DIR_NAME, MANIFEST_FILE_NAME};

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

fn mk(MkOpts { path, name }: MkOpts, _config: &Config) -> Result<()> {
    // Create project directory in case we are called from `new` op.
    fsx::create_dir_all(&path)?;

    // Write VCS ignore file.
    // TODO(mkaput): Print a message to the user that they need to add `target` themself.
    let gitignore = path.join(".gitignore");
    if !gitignore.exists() {
        fsx::write(gitignore, DEFAULT_TARGET_DIR_NAME)?;
    }

    // Create the `Scarb.toml` file.
    fsx::write(
        path.join(MANIFEST_FILE_NAME),
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

    // TODO(mkaput): Take this from Cargo:
    //   if let Err(e) = Workspace::new(&path.join("Cargo.toml"), config) {
    //       crate::display_warning_with_error(
    //           "compiling this new package may not work due to invalid \
    //            workspace configuration",
    //           &e,
    //           &mut config.shell(),
    //       );
    //   }

    Ok(())
}
