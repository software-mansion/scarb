use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use camino::Utf8PathBuf;
use semver::Version;
use toml_edit::{DocumentMut, Item, Value};

use scarb_build_metadata::CAIRO_VERSION;
use to_version::ToVersion;

use crate::fsx::{PathUtf8Ext, make_executable};
use crate::gitx::GitProject;

#[path = "../../../scarb/src/internal/to_version.rs"]
mod to_version;

pub struct ProjectBuilder {
    name: String,
    version: Version,
    edition: Option<String>,
    cairo_version: Option<Version>,
    src: HashMap<Utf8PathBuf, Vec<u8>>,
    deps: Vec<(String, Value)>,
    dev_deps: Vec<(String, Value)>,
    manifest_package_extra: String,
    manifest_extra: String,
    executable_files: Vec<Utf8PathBuf>,
}

impl ProjectBuilder {
    pub fn start() -> Self {
        // Nondeterministic counter forces users to fix fields that are present in assertions.
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let n = COUNTER.fetch_add(0, Ordering::Relaxed);
        Self {
            name: format!("pkg{n}"),
            version: Version::new(1, n, 0),
            edition: "2024_07".to_string().into(),
            cairo_version: None,
            src: HashMap::from_iter([(
                Utf8PathBuf::from("src/lib.cairo"),
                format!(r#"fn f{n}() -> felt252 {{ {n} }}"#).into(),
            )]),
            deps: Vec::new(),
            dev_deps: Vec::new(),
            manifest_package_extra: String::new(),
            manifest_extra: String::new(),
            executable_files: Vec::new(),
        }
    }

    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn version(mut self, version: impl ToVersion) -> Self {
        self.version = version.to_version().unwrap();
        self
    }

    pub fn edition(mut self, edition: impl ToString) -> Self {
        self.edition = Some(edition.to_string());
        self
    }

    pub fn no_edition(mut self) -> Self {
        self.edition = None;
        self
    }

    pub fn cairo_version(mut self, cairo_version: impl ToVersion) -> Self {
        self.cairo_version = Some(cairo_version.to_version().unwrap());
        self
    }

    pub fn src(self, path: impl Into<Utf8PathBuf>, source: impl ToString) -> Self {
        self.src_binary(path, source.to_string())
    }

    pub fn src_binary(mut self, path: impl Into<Utf8PathBuf>, source: impl Into<Vec<u8>>) -> Self {
        self.src.insert(path.into(), source.into());
        self
    }

    pub fn lib_cairo(self, source: impl ToString) -> Self {
        self.src("src/lib.cairo", source.to_string())
    }

    pub fn lock(self, source: impl ToString) -> Self {
        self.src("Scarb.lock", source.to_string())
    }

    pub fn cp(self, from: impl AsRef<Path>, to: impl Into<Utf8PathBuf>) -> Self {
        self.src(to, std::fs::read_to_string(from.as_ref()).unwrap())
    }

    pub fn dep(mut self, name: impl ToString, dep: impl DepBuilder) -> Self {
        self.deps.push((name.to_string(), dep.build()));
        self
    }

    pub fn dev_dep(mut self, name: impl ToString, dep: impl DepBuilder) -> Self {
        self.dev_deps.push((name.to_string(), dep.build()));
        self
    }

    pub fn dep_builtin(self, name: impl ToString) -> Self {
        self.dep(name, Dep.version(CAIRO_VERSION))
    }

    pub fn dev_dep_builtin(self, name: impl ToString) -> Self {
        self.dev_dep(name, Dep.version(CAIRO_VERSION))
    }

    pub fn dep_starknet(self) -> Self {
        self.dep_builtin("starknet")
    }

    pub fn dep_cairo_execute(self) -> Self {
        self.dep_builtin("cairo_execute")
    }

    pub fn dep_cairo_test(self) -> Self {
        self.dev_dep_builtin("cairo_test")
    }

    pub fn dep_oracle_asserts(self) -> Self {
        self.dep("oracle_asserts", resource("oracle_asserts"))
    }

    pub fn manifest_package_extra(mut self, extra: impl ToString) -> Self {
        self.manifest_package_extra = extra.to_string();
        self
    }

    pub fn manifest_extra(mut self, extra: impl ToString) -> Self {
        self.manifest_extra = extra.to_string();
        self
    }

    pub fn make_executable(mut self, path: impl Into<Utf8PathBuf>) -> Self {
        self.executable_files.push(path.into());
        self
    }

    pub fn render_manifest(&self) -> String {
        let mut doc = DocumentMut::new();
        doc["package"] = toml_edit::table();
        doc["package"]["name"] = Item::Value(Value::from(self.name.clone()));
        doc["package"]["version"] = Item::Value(Value::from(self.version.to_string()));
        if let Some(edition) = self.edition.as_ref() {
            doc["package"]["edition"] = Item::Value(Value::from(edition.to_string()));
        }
        if let Some(cairo_version) = self.cairo_version.as_ref() {
            doc["package"]["cairo-version"] = Item::Value(Value::from(cairo_version.to_string()));
        }
        let mut manifest = doc.to_string();
        if !self.manifest_package_extra.is_empty() {
            manifest.push_str(&self.manifest_package_extra);
        }

        let mut doc = manifest.parse::<DocumentMut>().unwrap();
        doc["dependencies"] = toml_edit::table();
        for (name, dep) in &self.deps {
            doc["dependencies"][name.clone()] = Item::Value(dep.clone());
        }
        if !self.dev_deps.is_empty() {
            doc["dev-dependencies"] = toml_edit::table();
            for (name, dep) in &self.dev_deps {
                doc["dev-dependencies"][name.clone()] = Item::Value(dep.clone());
            }
        }
        let mut manifest = doc.to_string();

        if !self.manifest_extra.is_empty() {
            manifest.push('\n');
            manifest.push_str(&self.manifest_extra);
        }
        manifest
    }

    pub fn just_manifest(&self, t: &impl PathChild) {
        let manifest = self.render_manifest();
        t.child("Scarb.toml").write_str(&manifest).unwrap();
    }

    pub fn just_code(&self, t: &impl PathChild) {
        for (path, source) in &self.src {
            t.child(path).write_binary(source).unwrap();
        }

        for path in &self.executable_files {
            make_executable(t.child(path).path());
        }
    }

    pub fn build(&self, t: &impl PathChild) {
        self.just_manifest(t);
        self.just_code(t);
    }
}

pub trait DepBuilder {
    fn build(&self) -> Value;

    fn with(&self, key: impl ToString, value: impl Into<Value>) -> DepWith<'_, Self> {
        DepWith {
            dep: self,
            key: key.to_string(),
            value: value.into(),
        }
    }

    fn version(&self, version: impl ToString) -> DepWith<'_, Self> {
        self.with("version", version.to_string())
    }

    fn workspace(&self) -> DepWith<'_, Self> {
        self.with("workspace", true)
    }

    fn path(&self, path: impl ToString) -> DepWith<'_, Self> {
        self.with("path", path.to_string())
    }

    // Taking by reference to disallow dropping the `LocalRegistry`.
    fn registry(&self, registry: &impl ToString) -> DepWith<'_, Self> {
        self.with("registry", registry.to_string())
    }

    fn features(&self, features: impl Iterator<Item = impl ToString>) -> DepWith<'_, Self> {
        let features = Value::Array(toml_edit::Array::from_iter(features.map(|v| v.to_string())));
        self.with("features", features)
    }

    fn default_features(&self, default_features: bool) -> DepWith<'_, Self> {
        self.with("default-features", default_features)
    }
}

pub struct Dep;

impl DepBuilder for Dep {
    fn build(&self) -> Value {
        toml_edit::table().into_value().unwrap()
    }
}

impl DepBuilder for &ChildPath {
    fn build(&self) -> Value {
        ChildPath::path(self).build()
    }
}

impl DepBuilder for ChildPath {
    fn build(&self) -> Value {
        (&self).build()
    }
}

impl DepBuilder for &GitProject {
    fn build(&self) -> Value {
        Dep.with("git", self.url()).build()
    }
}

impl DepBuilder for GitProject {
    fn build(&self) -> Value {
        (&self).build()
    }
}

impl DepBuilder for &Path {
    fn build(&self) -> Value {
        Dep.with("path", self.try_to_utf8().unwrap().to_string())
            .build()
    }
}

impl DepBuilder for Path {
    fn build(&self) -> Value {
        (&self).build()
    }
}

impl DepBuilder for &PathBuf {
    fn build(&self) -> Value {
        self.as_path().build()
    }
}

impl DepBuilder for PathBuf {
    fn build(&self) -> Value {
        self.as_path().build()
    }
}

pub struct DepWith<'a, T: DepBuilder + ?Sized> {
    dep: &'a T,
    key: String,
    value: Value,
}

impl<T: DepBuilder + ?Sized> DepBuilder for DepWith<'_, T> {
    fn build(&self) -> Value {
        let mut table = self.dep.build();
        table
            .as_inline_table_mut()
            .unwrap()
            .insert(self.key.clone(), self.value.clone());
        table
    }
}

fn resource(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}
