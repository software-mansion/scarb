use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::core::registry::index::IndexRecords;
use crate::core::{PackageId, PackageName};

pub mod local;

#[async_trait]
pub trait RegistryClient: Send + Sync {
    /// State whether this registry works in offline mode.
    ///
    /// Local registries are expected to perform immediate file operations, while remote registries
    /// can take some IO-bound time. This flag also influences appearance of various UI elements.
    fn is_offline(&self) -> bool;

    /// Get the index record for a specific named package from this index.
    ///
    /// Returns `None` if the package is not present in the index.
    ///
    /// ## Caching
    ///
    /// This method is not expected to internally cache the result, but it is not prohibited either.
    /// Scarb applies specialized caching layers on top of clients.
    async fn get_records(&self, package: PackageName) -> Result<Option<Arc<IndexRecords>>>;

    /// Check if the package `.tar.zst` file has already been downloaded and is stored on disk.
    ///
    /// On internal errors, this method should return `false`. This method must not perform any
    /// network operations (it can be called before offline mode check).
    async fn is_downloaded(&self, package: PackageId) -> bool;

    /// Download the package `.tar.zst` file.
    ///
    /// Returns a [`PathBuf`] to the downloaded `.tar.zst` file.
    ///
    /// ## Caching
    ///
    /// If the registry is remote, i.e. actually downloads files and writes them to disk,
    /// it should write downloaded files to Scarb cache directory. If the file has already been
    /// downloaded, it should avoid downloading it again, and read it from this cache instead.
    async fn download(&self, package: PackageId) -> Result<PathBuf>;
}
