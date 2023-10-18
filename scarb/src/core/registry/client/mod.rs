use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::core::registry::index::IndexRecords;
use crate::core::{Package, PackageId, PackageName};
use crate::flock::FileLockGuard;

pub mod http;
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

    /// State whether packages can be published to this registry.
    ///
    /// This method is permitted to do network lookups, for example to fetch registry config.
    async fn supports_publish(&self) -> Result<bool> {
        Ok(false)
    }

    /// Publish a package to this registry.
    ///
    /// This function can only be called if [`RegistryClient::supports_publish`] returns `true`.
    /// Default implementation panics with [`unreachable!`].
    ///
    /// The `package` argument must correspond to just packaged `tarball` file.
    /// The client is free to use information within `package` to send to the registry.
    /// Package source is not required to match the registry the package is published to.
    async fn publish(&self, package: Package, tarball: FileLockGuard) -> Result<()> {
        // Silence clippy warnings without using _ in argument names.
        let _ = package;
        let _ = tarball;
        unreachable!("This registry does not support publishing.")
    }
}
