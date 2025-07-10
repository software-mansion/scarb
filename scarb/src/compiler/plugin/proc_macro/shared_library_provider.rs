use crate::compiler::CompilationUnitCairoPlugin;
use crate::compiler::plugin::proc_macro::compilation::{
    PROC_MACRO_BUILD_PROFILE, get_cargo_package_name,
};
use crate::core::{Config, Package};
use crate::flock::Filesystem;
use anyhow::{Context, anyhow};
use camino::Utf8PathBuf;
use cargo_metadata::MetadataCommand;
use indoc::formatdoc;
use libloading::library_filename;
use ra_ap_toolchain::Tool;
use std::env::consts::DLL_SUFFIX;
use target_triple::target;

/// This trait is used to define the target and prebuilt path for a package.
pub trait ProcMacroPathsProvider {
    /// Location of Cargo `target` directory.
    fn target_path(&self, config: &Config) -> Filesystem;
    /// Location of the prebuilt binary for the package, if defined.
    fn prebuilt_lib_path(&self) -> Option<Utf8PathBuf>;
}

pub trait SharedLibraryProvider {
    /// Location of the shared library for the package.
    fn shared_lib_path(&self, config: &Config) -> anyhow::Result<Utf8PathBuf>;
}

impl ProcMacroPathsProvider for Package {
    fn target_path(&self, config: &Config) -> Filesystem {
        let ident = format!("{}-{}", self.id.name, self.id.source_id.ident());
        // Defines the Cargo target directory in cache, as:
        // `/(..)/SCARB_CACHE/plugins/proc_macro/<package_name>-<source_id_ident>/v<version>/target/`
        config
            .dirs()
            .procedural_macros_dir()
            .into_child(ident)
            .into_child(format!("v{}", self.id.version))
            .into_child("target")
    }
    fn prebuilt_lib_path(&self) -> Option<Utf8PathBuf> {
        let target_triple = target!();

        let prebuilt_name = format!(
            "{name}_v{version}_{target}{suffix}",
            name = self.id.name,
            version = self.id.version,
            target = target_triple,
            suffix = DLL_SUFFIX
        );

        let prebuilt_path = self
            .root()
            .join("target")
            .join("scarb")
            .join("cairo-plugin")
            .join(prebuilt_name);

        prebuilt_path.exists().then_some(prebuilt_path)
    }
}

impl SharedLibraryProvider for CompilationUnitCairoPlugin {
    fn shared_lib_path(&self, config: &Config) -> anyhow::Result<Utf8PathBuf> {
        if let Some(path) = self.cached_shared_lib_path.get() {
            return Ok(path.clone());
        }

        let lib_name = get_cargo_library_name(&self.package, config)
            .context("could not resolve library name")?;
        let lib_name = library_filename(lib_name);
        let lib_name = lib_name
            .into_string()
            .expect("library name must be valid UTF-8");
        // Defines the shared library path inside the target directory, as:
        // `/(..)/target/release/[lib]<package_name>.[so|dll|dylib]`
        let path = self
            .package
            .target_path(config)
            .into_child(PROC_MACRO_BUILD_PROFILE)
            .path_unchecked()
            .join(lib_name);

        Ok(self
            .cached_shared_lib_path
            // Note we don't just wrap everything in `get_or_init`, but check the cache manually,
            // to make the error handling a bit less awkward.
            .get_or_init(|| path.clone())
            .clone())
    }
}

pub fn get_cargo_library_name(package: &Package, config: &Config) -> anyhow::Result<String> {
    let metadata = MetadataCommand::new()
        .cargo_path(Tool::Cargo.path())
        .current_dir(package.root())
        .exec()
        .context("could not get Cargo metadata")?;

    let cargo_package_name = get_cargo_package_name(package)?;

    if cargo_package_name != package.id.name.to_string() {
        config.ui().warn(formatdoc!(
            r#"
            package name differs between Cargo and Scarb manifest
            cargo: `{cargo_name}`, scarb: `{scarb_name}`
            this might become an error in future Scarb releases
            "#,
            cargo_name = cargo_package_name,
            scarb_name = package.id.name,
        ));
    }

    let package = metadata
        .packages
        .iter()
        .find(|pkg| pkg.name.as_str() == cargo_package_name)
        .ok_or_else(|| anyhow!("could not get `{cargo_package_name}` package from metadata"))?;

    let cdylib_target = package
        .targets
        .iter()
        .find(|target| target.kind.contains(&"cdylib".into()))
        .ok_or_else(|| anyhow!("no target of `cdylib` kind found in package"))?;

    Ok(cdylib_target.name.clone())
}
