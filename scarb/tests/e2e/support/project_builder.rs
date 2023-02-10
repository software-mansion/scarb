use std::sync::atomic::{AtomicU64, Ordering};

use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use semver::Version;
use toml_edit::{Document, Item, Table, Value};

use to_version::ToVersion;

use crate::support::fsx::PathUtf8Ext;
use crate::support::gitx::GitProject;

#[path = "../../../src/internal/to_version.rs"]
mod to_version;

pub struct ProjectBuilder {
    name: String,
    version: Version,
    lib_cairo: String,
    deps: Vec<(String, Value)>,
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
            deps: Vec::new(),
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

    pub fn dep(mut self, name: impl Into<String>, dep: impl ToDep) -> Self {
        self.deps.push((name.into(), dep.to_dep()));
        self
    }

    pub fn just_manifest(&self, t: &impl PathChild) {
        let mut doc = Document::new();
        doc["package"]["name"] = Item::Value(Value::from(self.name.clone()));
        doc["package"]["version"] = Item::Value(Value::from(self.version.to_string()));

        for (name, dep) in &self.deps {
            doc["dependencies"][name.clone()] = Item::Value(dep.clone());
        }

        let manifest = doc.to_string();
        t.child("Scarb.toml").write_str(&manifest).unwrap();
    }

    pub fn just_code(&self, t: &impl PathChild) {
        t.child("src/lib.cairo").write_str(&self.lib_cairo).unwrap();
    }

    pub fn build(&self, t: &impl PathChild) {
        self.just_manifest(t);

        self.just_code(t);
    }
}

pub trait ToDep {
    fn to_dep(&self) -> Value;
}

impl ToDep for Table {
    fn to_dep(&self) -> Value {
        self.clone().into_inline_table().into()
    }
}

impl ToDep for &str {
    fn to_dep(&self) -> Value {
        let doc = self.parse::<Document>().unwrap();
        let tab = doc.as_table().clone().into_inline_table();
        Value::InlineTable(tab)
    }
}

impl ToDep for String {
    fn to_dep(&self) -> Value {
        self.as_str().to_dep()
    }
}

impl ToDep for &ChildPath {
    fn to_dep(&self) -> Value {
        let mut table = toml_edit::table();
        table["path"] = Item::Value(Value::from(self.path().try_to_utf8().unwrap().to_string()));
        table.into_value().unwrap()
    }
}

impl ToDep for &GitProject {
    fn to_dep(&self) -> Value {
        let mut table = toml_edit::table();
        table["git"] = Item::Value(Value::from(self.url()));
        table.into_value().unwrap()
    }
}
