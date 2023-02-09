use std::sync::atomic::{AtomicU64, Ordering};

use assert_fs::prelude::*;
use indoc::formatdoc;
use semver::Version;

use to_version::ToVersion;

#[path = "../../../src/internal/to_version.rs"]
mod to_version;

pub struct ProjectBuilder {
    name: String,
    version: Version,
    lib_cairo: String,
}

impl ProjectBuilder {
    pub fn start() -> Self {
        // Nondeterministic counter forces users to fix fields that are present in assertions.
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let n = COUNTER.fetch_add(0, Ordering::Relaxed);
        Self {
            name: format!("pkg{n}"),
            version: Version::new(1, n, 0),
            lib_cairo: format!(r#"fn f{n}() -> felt {{ {n} }}"#),
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn version(mut self, version: impl ToVersion) -> Self {
        self.version = version.to_version().unwrap();
        self
    }

    pub fn lib_cairo(mut self, lib_cairo: impl Into<String>) -> Self {
        self.lib_cairo = lib_cairo.into();
        self
    }

    pub fn just_manifest(&self, t: &impl PathChild) {
        let Self { name, version, .. } = self;

        t.child("Scarb.toml")
            .write_str(&formatdoc! {r#"
                [package]
                name = "{name}"
                version = "{version}"
            "#})
            .unwrap();
    }

    pub fn just_code(&self, t: &impl PathChild) {
        t.child("src/lib.cairo").write_str(&self.lib_cairo).unwrap();
    }

    pub fn build(&self, t: &impl PathChild) {
        self.just_manifest(t);

        self.just_code(t);
    }
}
