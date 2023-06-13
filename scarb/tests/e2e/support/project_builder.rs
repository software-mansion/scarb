use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use camino::Utf8PathBuf;
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
    cairo_version: Option<Version>,
    src: HashMap<Utf8PathBuf, String>,
    deps: Vec<(String, Value)>,
    manifest_extra: String,
}

impl ProjectBuilder {
    pub fn start() -> Self {
        // Nondeterministic counter forces users to fix fields that are present in assertions.
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let n = COUNTER.fetch_add(0, Ordering::Relaxed);
        Self {
            name: format!("pkg{n}"),
            version: Version::new(1, n, 0),
            cairo_version: None,
            src: HashMap::from_iter([(
                Utf8PathBuf::from("src/lib.cairo"),
                format!(r#"fn f{n}() -> felt252 {{ {n} }}"#),
            )]),
            deps: Vec::new(),
            manifest_extra: String::new(),
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

    pub fn cairo_version(mut self, cairo_version: impl ToVersion) -> Self {
        self.cairo_version = Some(cairo_version.to_version().unwrap());
        self
    }

    pub fn src(mut self, path: impl Into<Utf8PathBuf>, source: impl Into<String>) -> Self {
        self.src.insert(path.into(), source.into());
        self
    }

    pub fn lib_cairo(self, source: impl Into<String>) -> Self {
        self.src("src/lib.cairo", source.into())
    }

    pub fn dep(mut self, name: impl Into<String>, dep: impl ToDep) -> Self {
        self.deps.push((name.into(), dep.to_dep()));
        self
    }

    pub fn dep_starknet(self) -> Self {
        self.dep("starknet", r#"version = ">=2.0.0-rc0""#)
    }

    pub fn manifest_extra(mut self, extra: impl Into<String>) -> Self {
        self.manifest_extra = extra.into();
        self
    }

    pub fn just_manifest(&self, t: &impl PathChild) {
        let mut doc = Document::new();
        doc["package"]["name"] = Item::Value(Value::from(self.name.clone()));
        doc["package"]["version"] = Item::Value(Value::from(self.version.to_string()));
        if let Some(cairo_version) = self.cairo_version.as_ref() {
            doc["package"]["cairo-version"] = Item::Value(Value::from(cairo_version.to_string()));
        }
        for (name, dep) in &self.deps {
            doc["dependencies"][name.clone()] = Item::Value(dep.clone());
        }

        let mut manifest = doc.to_string();

        if !self.manifest_extra.is_empty() {
            manifest.push('\n');
            manifest.push_str(&self.manifest_extra);
        }

        t.child("Scarb.toml").write_str(&manifest).unwrap();
    }

    pub fn just_code(&self, t: &impl PathChild) {
        for (path, source) in &self.src {
            t.child(path).write_str(source).unwrap();
        }
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
