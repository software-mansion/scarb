use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};

pub use id::*;
pub use name::*;

use crate::core::manifest::Manifest;
use crate::core::Target;
use crate::DEFAULT_SOURCE_DIR_NAME;

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

    pub fn source_dir(&self) -> Utf8PathBuf {
        self.root().join(DEFAULT_SOURCE_DIR_NAME)
    }

    pub fn is_lib(&self) -> bool {
        self.manifest.targets.iter().any(Target::is_lib)
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}
