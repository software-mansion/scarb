use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{env, mem};

use anyhow::{anyhow, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use once_cell::sync::OnceCell;
use tokio::runtime::{Builder, Handle, Runtime};
use tracing::trace;
use which::which_in;

use scarb_ui::{OutputFormat, Ui, Verbosity};

use crate::compiler::plugin::CairoPluginRepository;
use crate::compiler::{CompilerRepository, Profile};
use crate::core::AppDirs;
#[cfg(doc)]
use crate::core::Workspace;
use crate::flock::{AdvisoryLock, RootFilesystem};
use crate::internal::fsx;
use crate::DEFAULT_TARGET_DIR_NAME;
use crate::SCARB_ENV;

pub struct Config {
    manifest_path: Utf8PathBuf,
    dirs: Arc<AppDirs>,
    target_dir: RootFilesystem,
    app_exe: OnceCell<PathBuf>,
    ui: Ui,
    creation_time: Instant,
    // HACK: This should be the lifetime of Config itself, but we cannot express that, so we
    //   put static lifetime here and transmute in getter function.
    package_cache_lock: OnceCell<AdvisoryLock<'static>>,
    log_filter_directive: OsString,
    offline: bool,
    compilers: CompilerRepository,
    cairo_plugins: CairoPluginRepository,
    tokio_runtime: OnceCell<Runtime>,
    tokio_handle: OnceCell<Handle>,
    profile: Profile,
}

impl Config {
    pub fn builder(manifest_path: impl Into<Utf8PathBuf>) -> ConfigBuilder {
        ConfigBuilder::new(manifest_path.into())
    }

    fn build(b: ConfigBuilder) -> Result<Self> {
        let creation_time = Instant::now();

        let ui = Ui::new(b.ui_verbosity, b.ui_output_format);

        let dirs = Arc::new(AppDirs::init(
            b.global_cache_dir_override,
            b.global_config_dir_override,
            b.path_env_override,
        )?);

        if tracing::enabled!(tracing::Level::TRACE) {
            for line in dirs.to_string().lines() {
                trace!("{line}");
            }
        }

        let target_dir =
            RootFilesystem::new_output_dir(b.target_dir_override.unwrap_or_else(|| {
                b.manifest_path
                    .parent()
                    .expect("parent of manifest path must always exist")
                    .join(DEFAULT_TARGET_DIR_NAME)
            }));

        let compilers = b.compilers.unwrap_or_else(CompilerRepository::std);
        let compiler_plugins = b.cairo_plugins.unwrap_or_else(CairoPluginRepository::std);
        let profile: Profile = b.profile.unwrap_or_default();
        let tokio_handle: OnceCell<Handle> = OnceCell::new();
        if let Some(handle) = b.tokio_handle {
            tokio_handle.set(handle).unwrap();
        }

        Ok(Self {
            manifest_path: b.manifest_path,
            dirs,
            target_dir,
            app_exe: OnceCell::new(),
            ui,
            creation_time,
            package_cache_lock: OnceCell::new(),
            log_filter_directive: b.log_filter_directive.unwrap_or_default(),
            offline: b.offline,
            compilers,
            cairo_plugins: compiler_plugins,
            tokio_runtime: OnceCell::new(),
            tokio_handle,
            profile,
        })
    }

    pub fn manifest_path(&self) -> &Utf8Path {
        &self.manifest_path
    }

    pub fn root(&self) -> &Utf8Path {
        self.manifest_path()
            .parent()
            .expect("parent of manifest path must always exist")
    }

    pub fn log_filter_directive(&self) -> &OsStr {
        &self.log_filter_directive
    }

    pub fn dirs(&self) -> &AppDirs {
        &self.dirs
    }

    pub fn target_dir(&self) -> &RootFilesystem {
        &self.target_dir
    }

    pub fn app_exe(&self) -> Result<&Path> {
        self.app_exe
            .get_or_try_init(|| {
                let from_env = || -> Result<PathBuf> {
                    // Try re-using the `scarb` set in the environment already.
                    // This allows commands that use Scarb as a library to inherit
                    // (via `scarb <subcommand>`) or set (by setting `$SCARB`) a correct path
                    // to `scarb` when the current exe is not actually scarb (e.g. `scarb-*` binaries).
                    let path = env::var_os(SCARB_ENV)
                        .map(PathBuf::from)
                        .ok_or_else(|| anyhow!("${SCARB_ENV} not set"))?;
                    let path = fsx::canonicalize(path)?;
                    Ok(path)
                };

                let from_current_exe = || -> Result<PathBuf> {
                    // Try fetching the path to `scarb` using `env::current_exe()`.
                    // The method varies per operating system and might fail; in particular,
                    // it depends on `/proc` being mounted on Linux, and some environments
                    // (like containers or chroots) may not have that available.
                    let path = env::current_exe()?;
                    let path = fsx::canonicalize(path)?;
                    Ok(path)
                };

                let from_argv = || -> Result<PathBuf> {
                    // Grab `argv[0]` and attempt to resolve it to an absolute path.
                    // If `argv[0]` has one component, it must have come from a `PATH` lookup,
                    // so probe `PATH` in that case.
                    // Otherwise, it has multiple components and is either:
                    // - a relative path (e.g., `./scarb`, `target/debug/scarb`), or
                    // - an absolute path (e.g., `/usr/local/bin/scarb`).
                    // In either case, [`fsx::canonicalize`] will return the full absolute path
                    // to the target if it exists.
                    let argv0 = env::args_os()
                        .map(PathBuf::from)
                        .next()
                        .ok_or_else(|| anyhow!("no argv[0]"))?;
                    which_in(argv0, Some(self.dirs().path_env()), env::current_dir()?)
                        .map_err(Into::into)
                };

                from_env()
                    .or_else(|_| from_current_exe())
                    .or_else(|_| from_argv())
                    .context("could not get the path to scarb executable")
            })
            .map(AsRef::as_ref)
    }

    pub fn ui(&self) -> &Ui {
        &self.ui
    }

    pub fn elapsed_time(&self) -> Duration {
        self.creation_time.elapsed()
    }

    pub fn package_cache_lock<'a>(&'a self) -> &AdvisoryLock<'a> {
        // UNSAFE: These mem::transmute calls only change generic lifetime parameters.
        let static_al: &AdvisoryLock<'static> = self.package_cache_lock.get_or_init(|| {
            let not_static_al =
                self.dirs()
                    .cache_dir
                    .advisory_lock(".package-cache.lock", "package cache", self);
            unsafe { mem::transmute(not_static_al) }
        });
        let not_static_al: &AdvisoryLock<'a> = unsafe { mem::transmute(static_al) };
        not_static_al
    }

    pub fn tokio_handle(&self) -> &Handle {
        self.tokio_handle.get_or_init(|| {
            // No tokio runtime handle stored yet.
            if let Ok(handle) = Handle::try_current() {
                // Check if we're already in a tokio runtime.
                handle
            } else {
                // Otherwise, start a new one.
                let runtime = self
                    .tokio_runtime
                    .get_or_init(|| Builder::new_multi_thread().enable_all().build().unwrap());

                runtime.handle().clone()
            }
        })
    }

    /// States whether the _Offline Mode_ is turned on.
    ///
    /// For checking whether Scarb can communicate with the network, prefer to use
    /// [`Self::network_allowed`], as it might pull information from other sources in the future.
    pub const fn offline(&self) -> bool {
        self.offline
    }

    /// If `false`, Scarb should never access the network, but otherwise it should continue operating
    /// if possible.
    pub const fn network_allowed(&self) -> bool {
        !self.offline()
    }

    pub fn compilers(&self) -> &CompilerRepository {
        &self.compilers
    }

    pub fn cairo_plugins(&self) -> &CairoPluginRepository {
        &self.cairo_plugins
    }

    pub fn profile(&self) -> Profile {
        self.profile.clone()
    }
}

#[derive(Debug)]
pub struct ConfigBuilder {
    manifest_path: Utf8PathBuf,
    global_config_dir_override: Option<Utf8PathBuf>,
    global_cache_dir_override: Option<Utf8PathBuf>,
    path_env_override: Option<Vec<PathBuf>>,
    target_dir_override: Option<Utf8PathBuf>,
    ui_verbosity: Verbosity,
    ui_output_format: OutputFormat,
    offline: bool,
    log_filter_directive: Option<OsString>,
    compilers: Option<CompilerRepository>,
    cairo_plugins: Option<CairoPluginRepository>,
    tokio_handle: Option<Handle>,
    profile: Option<Profile>,
}

impl ConfigBuilder {
    fn new(manifest_path: Utf8PathBuf) -> Self {
        Self {
            manifest_path,
            global_config_dir_override: None,
            global_cache_dir_override: None,
            path_env_override: None,
            target_dir_override: None,
            ui_verbosity: Verbosity::Normal,
            ui_output_format: OutputFormat::Text,
            offline: false,
            log_filter_directive: None,
            compilers: None,
            cairo_plugins: None,
            tokio_handle: None,
            profile: None,
        }
    }

    pub fn build(self) -> Result<Config> {
        Config::build(self)
    }

    pub fn global_config_dir_override(
        mut self,
        global_config_dir_override: Option<impl Into<Utf8PathBuf>>,
    ) -> Self {
        self.global_config_dir_override = global_config_dir_override.map(Into::into);
        self
    }

    pub fn global_cache_dir_override(
        mut self,
        global_cache_dir_override: Option<impl Into<Utf8PathBuf>>,
    ) -> Self {
        self.global_cache_dir_override = global_cache_dir_override.map(Into::into);
        self
    }

    pub fn path_env_override(
        mut self,
        path_env_override: Option<impl IntoIterator<Item = impl Into<PathBuf>>>,
    ) -> Self {
        self.path_env_override = path_env_override.map(|p| p.into_iter().map(Into::into).collect());
        self
    }

    pub fn target_dir_override(mut self, target_dir_override: Option<Utf8PathBuf>) -> Self {
        self.target_dir_override = target_dir_override;
        self
    }

    pub fn ui_verbosity(mut self, ui_verbosity: Verbosity) -> Self {
        self.ui_verbosity = ui_verbosity;
        self
    }

    pub fn ui_output_format(mut self, ui_output_format: OutputFormat) -> Self {
        self.ui_output_format = ui_output_format;
        self
    }

    pub fn offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    pub fn log_filter_directive(
        mut self,
        log_filter_directive: Option<impl Into<OsString>>,
    ) -> Self {
        self.log_filter_directive = log_filter_directive.map(Into::into);
        self
    }

    pub fn compilers(mut self, compilers: CompilerRepository) -> Self {
        self.compilers = Some(compilers);
        self
    }

    pub fn cairo_plugins(mut self, compiler_plugins: CairoPluginRepository) -> Self {
        self.cairo_plugins = Some(compiler_plugins);
        self
    }

    pub fn tokio_handle(mut self, tokio_handle: Handle) -> Self {
        self.tokio_handle = Some(tokio_handle);
        self
    }

    pub fn profile(mut self, profile: Profile) -> Self {
        self.profile = Some(profile);
        self
    }
}
