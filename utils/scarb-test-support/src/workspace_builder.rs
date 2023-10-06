use crate::project_builder::{DepBuilder, ProjectBuilder};
use assert_fs::prelude::*;
use scarb::MANIFEST_FILE_NAME;
use toml_edit::{Array, Document, Item, Value};

#[derive(Default)]
pub struct WorkspaceBuilder {
    members: Vec<String>,
    package: Option<ProjectBuilder>,
    manifest_extra: String,
    deps: Vec<(String, Value)>,
}

impl WorkspaceBuilder {
    pub fn start() -> Self {
        Self::default()
    }

    pub fn add_member(mut self, member: impl Into<String>) -> Self {
        self.members.push(member.into());
        self
    }

    pub fn package(mut self, package: ProjectBuilder) -> Self {
        self.package = Some(package);
        self
    }

    pub fn manifest_extra(mut self, extra: impl Into<String>) -> Self {
        self.manifest_extra = extra.into();
        self
    }

    pub fn dep(mut self, name: impl Into<String>, dep: impl DepBuilder) -> Self {
        self.deps.push((name.into(), dep.build()));
        self
    }

    pub fn build(&self, t: &impl PathChild) {
        let mut doc = Document::new();
        doc["workspace"] = toml_edit::table();
        doc["workspace"]["members"] =
            Item::Value(Value::from(Array::from_iter(self.members.clone())));
        doc["workspace"]["dependencies"] = toml_edit::table();
        for (name, dep) in &self.deps {
            doc["workspace"]["dependencies"][name.clone()] = Item::Value(dep.clone());
        }
        let mut manifest = doc.to_string();

        if let Some(package) = self.package.as_ref() {
            package.just_code(t);
            let pkg_manifest = package.render_manifest();
            manifest.push('\n');
            manifest.push_str(&pkg_manifest);
        }

        if !self.manifest_extra.is_empty() {
            manifest.push('\n');
            manifest.push_str(&self.manifest_extra);
        }

        t.child(MANIFEST_FILE_NAME).write_str(&manifest).unwrap();
    }
}
