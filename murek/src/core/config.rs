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
use crate::MUREK_ENV;

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
            .join("target");
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
                    // Try re-using the `murek` set in the environment already.
                    // This allows commands that use Murek as a library to inherit
                    // (via `murek <subcommand>`) or set (by setting `$MUREK`) a correct path
                    // to `murek` when the current exe is not actually murek (e.g. `murek-*` binaries).
                    env::var_os(MUREK_ENV)
                        .map(PathBuf::from)
                        .ok_or_else(|| anyhow!("${MUREK_ENV} not set"))?
                        .canonicalize()
                        .map_err(Into::into)
                };

                let from_current_exe = || -> Result<PathBuf> {
                    // Try fetching the path to `murek` using `env::current_exe()`.
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
                    // - a relative path (e.g., `./murek`, `target/debug/murek`), or
                    // - an absolute path (e.g., `/usr/local/bin/murek`).
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
                    .context("could not get the path to murek executable")
            })
            .map(AsRef::as_ref)
    }
}
