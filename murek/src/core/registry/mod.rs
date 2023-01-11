use anyhow::Result;
use async_trait::async_trait;

use crate::core::{ManifestDependency, Package, PackageId, Summary};

pub mod cache;
pub mod source_map;

#[async_trait(?Send)]
pub trait Registry {
    /// Attempt to find the packages that match a dependency request.
    async fn query(&mut self, dependency: &ManifestDependency) -> Result<Vec<Summary>>;

    /// Fetch full package by its ID.
    async fn download(&mut self, package_id: PackageId) -> Result<Package>;
}
