use crate::internal::fsx::PathBufUtf8Ext;
use anyhow::{bail, ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use indoc::{formatdoc, indoc};
use itertools::Itertools;

use crate::core::{Config, PackageName};
use crate::internal::fsx;
use crate::{ops, DEFAULT_SOURCE_DIR_NAME, DEFAULT_TARGET_DIR_NAME, MANIFEST_FILE_NAME};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VersionControl {
    Git,
    NoVcs,
}

#[derive(Debug)]
pub struct InitOptions {
    pub path: Utf8PathBuf,
    pub name: Option<PackageName>,
    pub vcs: VersionControl,
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
            version_control: opts.vcs,
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
            version_control: opts.vcs,
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
    version_control: VersionControl,
}

fn mk(
    MkOpts {
        path,
        name,
        version_control,
    }: MkOpts,
    config: &Config,
) -> Result<()> {
    // Create project directory in case we are called from `new` op.
    fsx::create_dir_all(&path)?;

    let canonical_path = if let Ok(canonicalize_path) = fsx::canonicalize(&path) {
        canonicalize_path.try_into_utf8()?
    } else {
        path
    };

    init_vcs(&canonical_path, version_control)?;
    write_vcs_ignore(&canonical_path, config, version_control)?;

    // Create the `Scarb.toml` file.
    let manifest_path = canonical_path.join(MANIFEST_FILE_NAME);
    fsx::write(
        &manifest_path,
        formatdoc! {r#"
            [package]
            name = "{name}"
            version = "0.1.0"

            # See more keys and their definitions at https://docs.swmansion.com/scarb/docs/reference/manifest

            [dependencies]
            # foo = {{ path = "vendor/foo" }}
        "#},
    )?;

    // Create hello world source files (with respective parent directories) if source directory
    // does not exist.
    let source_dir = canonical_path.join(DEFAULT_SOURCE_DIR_NAME);
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

fn init_vcs(path: &Utf8Path, vcs: VersionControl) -> Result<()> {
    match vcs {
        VersionControl::Git => {
            if !path.join(".git").exists() {
                gix::init(path)?;
            }
        }
        VersionControl::NoVcs => {}
    }

    Ok(())
}

/// Write VCS ignore file.
fn write_vcs_ignore(path: &Utf8Path, config: &Config, vcs: VersionControl) -> Result<()> {
    let patterns = vec![DEFAULT_TARGET_DIR_NAME];

    let fp_ignore = match vcs {
        VersionControl::Git => path.join(".gitignore"),
        VersionControl::NoVcs => return Ok(()),
    };

    if !fp_ignore.exists() {
        let ignore = patterns.join("\n") + "\n";
        fsx::write(&fp_ignore, ignore)?;
    } else {
        let lines = patterns
            .into_iter()
            .map(|pat| format!("    {pat}"))
            .join("\n");
        config
            .ui()
            .warn(formatdoc! {r#"
                file `{fp_ignore}` already exists in this directory, ensure following patterns are ignored:

                {lines}
            "#});
    }

    Ok(())
}
