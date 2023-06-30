use crate::project_builder::ProjectBuilder;
use assert_fs::prelude::*;
use toml_edit::{Array, Document, Item, Value};

#[derive(Default)]
pub struct WorkspaceBuilder {
    members: Vec<String>,
    package: Option<ProjectBuilder>,
    manifest_extra: String,
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

    pub fn build(&self, t: &impl PathChild) {
        let mut doc = Document::new();
        doc["workspace"]["members"] = Item::Value(Value::from(Array::from_iter(
            self.members.clone().into_iter(),
        )));
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

        t.child("Scarb.toml").write_str(&manifest).unwrap();
    }
}
