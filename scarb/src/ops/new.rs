use anyhow::{bail, ensure, Context, Result};
use cairo_lang_filesystem::db::Edition;
use camino::{Utf8Path, Utf8PathBuf};
use indoc::{formatdoc, indoc};
use itertools::Itertools;

use crate::core::{edition_variant, Config, PackageName};
use crate::internal::fsx;
use crate::internal::restricted_names;
use crate::{ops, DEFAULT_SOURCE_PATH, DEFAULT_TARGET_DIR_NAME, MANIFEST_FILE_NAME};
use std::process::Command;

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
    pub snforge: bool,
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

    let name = infer_name(opts.name, &opts.path, config)?;

    mk(
        MkOpts {
            path: opts.path.clone(),
            name: name.clone(),
            version_control: opts.vcs,
            snforge: opts.snforge,
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

    let name = infer_name(opts.name, &opts.path, config)?;

    mk(
        MkOpts {
            path: opts.path,
            name: name.clone(),
            version_control: opts.vcs,
            snforge: opts.snforge,
        },
        config,
    )
    .with_context(|| format!("failed to create package: {name}"))?;

    Ok(NewResult { name })
}

fn infer_name(name: Option<PackageName>, path: &Utf8Path, config: &Config) -> Result<PackageName> {
    let name = if let Some(name) = name {
        name
    } else {
        let Some(file_name) = path.file_name() else {
            bail!(formatdoc! {r#"
                cannot infer package name from path: {path}
                help: use --name to override
            "#});
        };
        PackageName::try_new(file_name)?
    };

    if restricted_names::is_internal(name.as_str()) {
        config.ui().warn(formatdoc! {r#"
            the name `{name}` is a Scarb internal package, \
            it is recommended to use a different name to avoid problems
        "#});
    }

    if restricted_names::is_windows_restricted(name.as_str()) {
        if cfg!(windows) {
            bail!("cannot use name `{name}`, it is a Windows reserved filename");
        } else {
            config.ui().warn(formatdoc! {r#"
                the name `{name}` is a Windows reserved filename, \
                this package will not work on Windows platforms
            "#})
        }
    }

    Ok(name)
}

struct MkOpts {
    path: Utf8PathBuf,
    name: PackageName,
    version_control: VersionControl,
    snforge: bool,
}

fn mk(
    MkOpts {
        path,
        name,
        version_control,
        snforge,
    }: MkOpts,
    config: &Config,
) -> Result<()> {
    // Create project directory in case we are called from `new` op.
    fsx::create_dir_all(&path)?;

    let canonical_path = fsx::canonicalize_utf8(&path).unwrap_or(path);

    init_vcs(&canonical_path, version_control)?;
    write_vcs_ignore(&canonical_path, config, version_control)?;

    // Create the `Scarb.toml` file.
    let manifest_path = canonical_path.join(MANIFEST_FILE_NAME);
    let edition = edition_variant(Edition::latest());
    fsx::write(
        &manifest_path,
        formatdoc! {r#"
            [package]
            name = "{name}"
            version = "0.1.0"
            edition = "{edition}"

            # See more keys and their definitions at https://docs.swmansion.com/scarb/docs/reference/manifest.html

            [dependencies]
        "#},
    )?;

    // Create hello world source files (with respective parent directories) if none exist.
    let source_path = canonical_path.join(DEFAULT_SOURCE_PATH.as_path());
    if !source_path.exists() {
        fsx::create_dir_all(source_path.parent().unwrap())?;

        fsx::write(
            source_path,
            indoc! {r#"
                fn main() -> u32 {
                    fib(16)
                }

                fn fib(mut n: u32) -> u32 {
                    let mut a: u32 = 0;
                    let mut b: u32 = 1;
                    while n != 0 {
                        n = n - 1;
                        let temp = b;
                        b = a + b;
                        a = temp;
                    };
                    a
                }

                #[cfg(test)]
                mod tests {
                    use super::fib;

                    #[test]
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

    if snforge {
        init_snforge(name)?;
    }

    Ok(())
}

fn init_snforge(name: PackageName) -> Result<()> {
    Command::new("snforge")
        .arg("init")
        .arg(name.as_str())
        .status()?;

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
