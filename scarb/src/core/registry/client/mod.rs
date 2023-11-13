use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;

use crate::core::registry::index::IndexRecords;
use crate::core::{Package, PackageId, PackageName};
use crate::flock::FileLockGuard;

pub mod cache;
pub mod http;
pub mod local;

/// Result from loading data from a registry.
pub enum RegistryResource<T> {
    /// The requested resource was not found.
    NotFound,
    /// The cache is valid and the cached data should be used.
    #[allow(dead_code)]
    InCache,
    /// The cache is out of date, new data was downloaded and should be used from now on.
    Download {
        resource: T,
        /// Client-dependent opaque value used to determine whether resource is out of date.
        ///
        /// Returning `None` means that this client/resource is not cacheable.
        cache_key: Option<String>,
    },
}

#[async_trait]
pub trait RegistryClient: Send + Sync {
    /// Get the index record for a specific named package from this index.
    ///
    /// Returns `None` if the package is not present in the index.
    ///
    /// ## Caching
    ///
    /// This method is not expected to internally cache the result, but it is not prohibited either.
    /// Scarb applies specialized caching layers on top of clients.
    async fn get_records(&self, package: PackageName) -> Result<RegistryResource<IndexRecords>>;

    /// Download the package `.tar.zst` file.
    ///
    /// Returns a [`PathBuf`] to the downloaded `.tar.zst` file.
    ///
    /// ## Caching
    ///
    /// This method is not expected to internally cache the result, but it is not prohibited either.
    /// Scarb applies specialized caching layers on top of clients.
    async fn download(&self, package: PackageId) -> Result<RegistryResource<PathBuf>>;

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
