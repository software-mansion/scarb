use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use camino::{Utf8Path, Utf8PathBuf};
use serde::Deserialize;

pub use id::*;
pub use name::*;
use scarb_ui::args::WithManifestPath;

use crate::core::manifest::Manifest;
use crate::core::Target;

mod id;
mod name;

/// See [`PackageInner`] for public fields reference.
#[derive(Clone, Debug)]
pub struct Package(Arc<PackageInner>);

#[derive(Debug)]
#[non_exhaustive]
pub struct PackageInner {
    pub id: PackageId,
    pub manifest: Box<Manifest>,
    manifest_path: Utf8PathBuf,
}

impl Deref for Package {
    type Target = PackageInner;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PackageClass {
    Library,
    CairoPlugin,
    Other,
}

impl Package {
    pub fn new(id: PackageId, manifest_path: Utf8PathBuf, manifest: Box<Manifest>) -> Self {
        Self(Arc::new(PackageInner {
            id,
            manifest_path,
            manifest,
        }))
    }

    pub fn root(&self) -> &Utf8Path {
        self.manifest_path
            .parent()
            .expect("manifest path parent must always exist")
    }

    pub fn manifest_path(&self) -> &Utf8Path {
        &self.manifest_path
    }

    pub fn is_lib(&self) -> bool {
        self.manifest.targets.iter().any(Target::is_lib)
    }

    pub fn is_cairo_plugin(&self) -> bool {
        self.manifest.targets.iter().any(Target::is_cairo_plugin)
    }

    pub fn classify(&self) -> PackageClass {
        if self.is_cairo_plugin() {
            PackageClass::CairoPlugin
        } else if self.is_lib() {
            PackageClass::Library
        } else {
            PackageClass::Other
        }
    }

    pub fn target(&self, kind: &str) -> Option<&Target> {
        self.manifest.targets.iter().find(|t| t.kind == kind)
    }

    pub fn fetch_target(&self, kind: &str) -> Result<&Target> {
        self.target(kind)
            .ok_or_else(|| anyhow!("package `{self}` has no target `{kind}`"))
    }

    pub fn has_tool_metadata(&self, tool_name: &str) -> bool {
        self.tool_metadata(tool_name).is_some()
    }

    pub fn tool_metadata(&self, tool_name: &str) -> Option<&toml::Value> {
        self.manifest
            .metadata
            .tool_metadata
            .as_ref()?
            .get(tool_name)
    }

    pub fn fetch_tool_metadata(&self, tool_name: &str) -> Result<&toml::Value> {
        self.tool_metadata(tool_name)
            .ok_or_else(|| anyhow!("package manifest `{self}` has no [tool.{tool_name}] section"))
    }

    pub fn fetch_tool_metadata_as<T: Deserialize<'static>>(&self, tool_name: &str) -> Result<T> {
        let toml_value = self.fetch_tool_metadata(tool_name)?;
        let structured = toml_value.clone().try_into()?;
        Ok(structured)
    }
}

impl WithManifestPath for Package {
    fn manifest_path(&self) -> &Utf8Path {
        &self.manifest_path
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}
