use anyhow::{bail, ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use indoc::{formatdoc, indoc};
use itertools::Itertools;

use crate::core::{Config, PackageName};
use crate::internal::fsx;
use crate::internal::fsx::PathBufUtf8Ext;
use crate::{ops, DEFAULT_SOURCE_PATH, DEFAULT_TARGET_DIR_NAME, MANIFEST_FILE_NAME};

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
        formatdoc!(
            r#"
                destination `{}` already exists
                help: use `scarb init` to initialize the directory
            "#,
            opts.path
        )
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
    .with_context(|| format!("failed to create package `{name}` at: {}", opts.path))?;

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
    .with_context(|| format!("failed to create package: {name}"))?;

    Ok(NewResult { name })
}

fn infer_name(name: Option<PackageName>, path: &Utf8Path) -> Result<PackageName> {
    if let Some(name) = name {
        Ok(name)
    } else {
        let Some(file_name) = path.file_name() else {
            bail!(formatdoc! {r#"
                cannot infer package name from path: {path}
                help: use --name to override
            "#});
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

            # See more keys and their definitions at https://docs.swmansion.com/scarb/docs/reference/manifest.html

            [dependencies]
            # foo = {{ path = "vendor/foo" }}
        "#},
    )?;

    // Create hello world source files (with respective parent directories) if none exist.
    let source_path = canonical_path.join(DEFAULT_SOURCE_PATH);
    if !source_path.exists() {
        fsx::create_dir_all(source_path.parent().unwrap())?;

        fsx::write(
            source_path,
            indoc! {r#"
                fn main() -> felt252 {
                    fib(16)
                }

                fn fib(mut n: felt252) -> felt252 {
                    let mut a: felt252 = 0;
                    let mut b: felt252 = 1;
                    loop {
                        if n == 0 {
                            break a;
                        }
                        n = n - 1;
                        let temp = b;
                        b = a + b;
                        a = temp;
                    }
                }

                #[cfg(test)]
                mod tests {
                    use super::fib;

                    #[test]
                    #[available_gas(100000)]
                    fn it_works() {
                        assert(fib(16) == 987, 'it works!');
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
