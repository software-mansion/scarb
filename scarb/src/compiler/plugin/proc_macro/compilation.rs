use crate::compiler::plugin::proc_macro::PROC_MACRO_BUILD_PROFILE;
use crate::core::{Config, Package};
use crate::flock::Filesystem;
use camino::Utf8PathBuf;
use libloading::library_filename;

/// This trait is used to define the shared library path for a package.
pub trait SharedLibraryProvider {
    /// Location of Cargo `target` directory.
    fn target_path(&self, config: &Config) -> Filesystem;
    /// Location of the shared library for the package.
    fn shared_lib_path(&self, config: &Config) -> Utf8PathBuf;
}

impl SharedLibraryProvider for Package {
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

    fn shared_lib_path(&self, config: &Config) -> Utf8PathBuf {
        let lib_name = library_filename(self.id.name.to_string());
        let lib_name = lib_name
            .into_string()
            .expect("library name must be valid UTF-8");
        // Defines the shared library path inside the target directory, as:
        // `/(..)/target/release/[lib]<package_name>.[so|dll|dylib]`
        self.target_path(config)
            .into_child(PROC_MACRO_BUILD_PROFILE)
            .path_unchecked()
            .join(lib_name)
    }
}
