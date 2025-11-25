use anyhow::{Context, Result, bail, ensure};
use cairo_lang_filesystem::db::Edition;
use camino::{Utf8Path, Utf8PathBuf};
use indoc::{formatdoc, indoc};
use itertools::Itertools;
use std::collections::HashMap;
use which::which;

use crate::core::{Config, PackageName, edition_variant};
use crate::internal::fsx;
use crate::internal::restricted_names;
use crate::process::is_truthy_env;
use crate::subcommands::get_env_vars;
use crate::{DEFAULT_SOURCE_PATH, DEFAULT_TARGET_DIR_NAME, MANIFEST_FILE_NAME, ops};
use scarb_build_metadata::CAIRO_VERSION;
use std::process::{Command, Stdio};

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

    let canonical_path = fsx::canonicalize_utf8(&path).unwrap_or(path.clone());

    init_vcs(&canonical_path, version_control)?;
    write_vcs_ignore(&canonical_path, config, version_control)?;

    let empty_init = snforge || is_truthy_env("SCARB_INIT_EMPTY", false);
    let template = if empty_init {
        Template::empty(&name)
    } else {
        Template::no_runner(&name)
    };

    template.materialize(&canonical_path)?;

    if let Err(err) = ops::read_workspace(&canonical_path.join(MANIFEST_FILE_NAME), config) {
        config.ui().warn(formatdoc! {r#"
            compiling this new package may not work due to invalid workspace configuration

            {err:?}
        "#})
    }

    if snforge {
        init_snforge(name, canonical_path, config)?;
    }

    Ok(())
}

struct Template {
    src: HashMap<SourcePath, Vec<u8>>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum SourcePath {
    Overwrite(Utf8PathBuf),
    SkipDuplicated(Utf8PathBuf),
}

impl AsRef<Utf8Path> for SourcePath {
    fn as_ref(&self) -> &Utf8Path {
        match self {
            SourcePath::Overwrite(path) => path,
            SourcePath::SkipDuplicated(path) => path,
        }
    }
}

impl Template {
    fn empty(name: &PackageName) -> Self {
        let edition = edition_variant(Edition::latest());
        Self {
            src: HashMap::from_iter(vec![
                (SourcePath::Overwrite(Utf8PathBuf::from(MANIFEST_FILE_NAME)), formatdoc!(r#"
                    [package]
                    name = "{name}"
                    version = "0.1.0"
                    edition = "{edition}"

                    # See more keys and their definitions at https://docs.swmansion.com/scarb/docs/reference/manifest.html

                    [dependencies]
                "#).into()),
                (SourcePath::SkipDuplicated(Utf8PathBuf::from("src/lib.cairo")), "".into())
            ]),
        }
    }

    fn no_runner(name: &PackageName) -> Self {
        let edition = edition_variant(Edition::latest());
        let cairo_version = CAIRO_VERSION;
        Self {
            src: HashMap::from_iter(vec![
                (SourcePath::Overwrite(Utf8PathBuf::from(MANIFEST_FILE_NAME)), formatdoc!(r#"
                    [package]
                    name = "{name}"
                    version = "0.1.0"
                    edition = "{edition}"

                    # See more keys and their definitions at https://docs.swmansion.com/scarb/docs/reference/manifest.html

                    [executable]

                    [cairo]
                    enable-gas = false

                    [dependencies]
                    cairo_execute = "{cairo_version}"
                "#).into()),
                (SourcePath::SkipDuplicated(Utf8PathBuf::from(DEFAULT_SOURCE_PATH.as_path())), "mod hello_world;\n".into()),
                (SourcePath::SkipDuplicated(Utf8PathBuf::from("src/hello_world.cairo")), indoc! {r#"
                    #[executable]
                    fn main() {
                        println!("Hello, World!");
                    }
                "#}.into())
            ]),
        }
    }

    fn materialize(&self, source_path: &Utf8Path) -> Result<()> {
        for (content_path, content) in self.src.iter() {
            let path = source_path.join(content_path.as_ref());
            fsx::create_dir_all(path.parent().expect("file path must have a parent"))?;

            match content_path {
                SourcePath::SkipDuplicated(_path) if path.exists() => continue,
                _ => {}
            }

            fsx::write(&path, content)?;
        }
        Ok(())
    }
}

fn init_snforge(name: PackageName, root_dir: Utf8PathBuf, config: &Config) -> Result<()> {
    // Check if snforge binary is available
    if which("snforge").is_err() {
        bail!(indoc! {r#"
            snforge binary not found

            Starknet Foundry needs to be installed to set up a project with snforge.

            To install snforge, please visit:
            https://foundry-rs.github.io/starknet-foundry/getting-started/installation.html

            Alternatively, you can manually add snforge to an existing project by following:
            https://foundry-rs.github.io/starknet-foundry/getting-started/first-steps.html#using-snforge-with-existing-scarb-projects

            You can also create a project without a test runner using the `--test-runner none` flag.
        "#});
    }

    let mut process = Command::new("snforge")
        .arg("new")
        .args(["--name", name.as_str()])
        .arg("--overwrite")
        .arg(root_dir.as_str())
        .envs(get_env_vars(config, None)?)
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .spawn()
        .context("failed to spawn snforge")?;

    process.wait().context("failed to execute snforge")?;

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
