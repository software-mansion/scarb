use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use id::*;

use crate::core::manifest::Manifest;

mod id;

/// See [`PackageInner`] for public fields reference.
#[derive(Clone, Debug)]
pub struct Package(Arc<PackageInner>);

#[derive(Debug)]
#[non_exhaustive]
pub struct PackageInner {
    pub id: PackageId,
    pub manifest: Box<Manifest>,
    manifest_path: PathBuf,
}

impl Deref for Package {
    type Target = PackageInner;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl Package {
    pub fn new(id: PackageId, manifest_path: PathBuf, manifest: Box<Manifest>) -> Self {
        Self(Arc::new(PackageInner {
            id,
            manifest_path,
            manifest,
        }))
    }

    pub fn root(&self) -> &Path {
        self.manifest_path
            .parent()
            .expect("manifest path parent must always exist")
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn source_dir(&self) -> PathBuf {
        self.root().join("src")
    }
}
