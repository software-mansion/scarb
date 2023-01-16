use std::env;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use once_cell::sync::OnceCell;
use tracing::trace;
use which::which_in;

#[cfg(doc)]
use crate::core::Workspace;
use crate::dirs::AppDirs;
use crate::internal::fsx::{GuardedExistedPathBuf, GuardedExistedPathBufOpts};
use crate::DEFAULT_TARGET_DIR_NAME;
use crate::SCARB_ENV;

pub type TargetDir = GuardedExistedPathBuf<'static>;

#[derive(Debug)]
pub struct Config {
    pub manifest_path: PathBuf,
    pub dirs: AppDirs,
    pub target_dir: TargetDir,

    app_exe: OnceCell<PathBuf>,
}

impl Config {
    pub fn init(manifest_path: PathBuf, dirs: AppDirs) -> Result<Self> {
        if tracing::enabled!(tracing::Level::TRACE) {
            for line in format!("{dirs}").lines() {
                trace!("{line}");
            }
        }

        let target_dir_path = manifest_path
            .parent()
            .expect("parent of manifest path must always exist")
            .join(DEFAULT_TARGET_DIR_NAME);
        let target_dir = TargetDir::with_options(
            target_dir_path,
            GuardedExistedPathBufOpts {
                is_output_dir: true,
            },
        );

        Ok(Self {
            manifest_path,
            dirs,
            target_dir,
            app_exe: OnceCell::new(),
        })
    }

    pub fn root(&self) -> &Path {
        self.manifest_path
            .parent()
            .expect("parent of manifest path must always exist")
    }

    pub fn app_exe(&self) -> Result<&Path> {
        self.app_exe
            .get_or_try_init(|| {
                let from_env = || -> Result<PathBuf> {
                    // Try re-using the `scarb` set in the environment already.
                    // This allows commands that use Scarb as a library to inherit
                    // (via `scarb <subcommand>`) or set (by setting `$SCARB`) a correct path
                    // to `scarb` when the current exe is not actually scarb (e.g. `scarb-*` binaries).
                    env::var_os(SCARB_ENV)
                        .map(PathBuf::from)
                        .ok_or_else(|| anyhow!("${SCARB_ENV} not set"))?
                        .canonicalize()
                        .map_err(Into::into)
                };

                let from_current_exe = || -> Result<PathBuf> {
                    // Try fetching the path to `scarb` using `env::current_exe()`.
                    // The method varies per operating system and might fail; in particular,
                    // it depends on `/proc` being mounted on Linux, and some environments
                    // (like containers or chroots) may not have that available.
                    env::current_exe()?.canonicalize().map_err(Into::into)
                };

                let from_argv = || -> Result<PathBuf> {
                    // Grab `argv[0]` and attempt to resolve it to an absolute path.
                    // If `argv[0]` has one component, it must have come from a `PATH` lookup,
                    // so probe `PATH` in that case.
                    // Otherwise, it has multiple components and is either:
                    // - a relative path (e.g., `./scarb`, `target/debug/scarb`), or
                    // - an absolute path (e.g., `/usr/local/bin/scarb`).
                    // In either case, [`Path::canonicalize`] will return the full absolute path
                    // to the target if it exists.
                    let argv0 = env::args_os()
                        .map(PathBuf::from)
                        .next()
                        .ok_or_else(|| anyhow!("no argv[0]"))?;
                    which_in(argv0, Some(self.dirs.path_env()), env::current_dir()?)
                        .map_err(Into::into)
                };

                from_env()
                    .or_else(|_| from_current_exe())
                    .or_else(|_| from_argv())
                    .context("could not get the path to scarb executable")
            })
            .map(AsRef::as_ref)
    }
}
