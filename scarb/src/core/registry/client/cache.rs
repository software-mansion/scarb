use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use camino::Utf8Path;
use redb::{MultimapTableDefinition, ReadableMultimapTable, ReadableTable, TableDefinition};
use semver::Version;
use tokio::sync::OnceCell;
use tokio::task::block_in_place;
use tracing::trace;

use scarb_ui::Ui;

use crate::core::registry::client::{RegistryClient, RegistryResource};
use crate::core::registry::index::{IndexRecord, IndexRecords};
use crate::core::{Config, ManifestDependency, PackageId, SourceId};
use crate::internal::fsx;

// TODO(mkaput): Implement cache downloading.
// FIXME(mkaput): Avoid creating database if inner client does not trigger cache writes.
// FIXME(mkaput): We probably have to call db.compact() after all write txs we run in Scarb run.

/// Multimap: `package name -> (version, index records)`.
const RECORDS: MultimapTableDefinition<'_, &str, (&str, &[u8])> =
    MultimapTableDefinition::new("records");

/// Map: `package name -> index records cache key`.
///
/// Cache key as returned by wrapped [`RegistryClient`].
const RECORDS_CACHE_KEYS: TableDefinition<'_, &str, &str> =
    TableDefinition::new("records_cache_keys");

/// A caching layer on top of a [`RegistryClient`].
///
/// ## Database
///
/// It uses [`redb`] as a local key-value database, where this object stores the following:
/// 1. Multimap table `records`: mapping from _package name_ to all index records that came from
///    the registry to date.
///
///    On the disk, each record are stored as a pair of _package version_ and the record itself
///    serialized as minified JSON. This allows the cache to filter out records that do not match
///    requested dependency specification before deserializing the record itself, saving some
///    execution time (exact numbers are unknown, but Cargo suffered from the same problem, and it
///    implemented identical measures).
/// 2. Table `records_cache_keys`: which maps _package name_ to the last known _cache key_ returned
///    from the [`RegistryClient::get_records`] method call.
///
/// Database files are stored in the `$SCARB_GLOBAL_CACHE/registry/cache` directory. For each
/// `SourceId` a separate database file is maintained, named `{source_id.ident()}.v1.redb`.
/// In case a new database format is used, it should be saved in a `*.v2.redb` file and so on.
/// Old versions should be simply deleted, without using sophisticated migration logic (remember,
/// this is just a cache!) Also, if the database file appears to be corrupted, it is simply deleted
/// and recreated from scratch.
///
/// ## Workflow
///
/// Each wrapper method of this struct performs more or less the same flow of steps:
/// 1. Get existing cache key from the database if exists.
/// 2. Call actual [`RegistryClient`] method with found cache key (or `None`).
/// 3. If the method returned [`RegistryResource::NotFound`], then everything related to queried
///    resource is removed from the cache.
/// 4. Or, if the method returned [`RegistryResource::InCache`], then cached value is deserialized
///    and returned.
/// 5. Or, if the method returned [`RegistryResource::Download`], then new resource data is saved
///    in cache (replacing existing items) along with new cache key and returned to the caller.
pub struct RegistryClientCache<'c> {
    source_id: SourceId,
    client: Box<dyn RegistryClient + 'c>,
    db_cell: OnceCell<CacheDatabase>,
    config: &'c Config,
}

impl<'c> RegistryClientCache<'c> {
    pub fn new(
        source_id: SourceId,
        client: Box<dyn RegistryClient + 'c>,
        config: &'c Config,
    ) -> Result<Self> {
        Ok(Self {
            source_id,
            client,
            db_cell: OnceCell::new(),
            config,
        })
    }

    /// Layer over [`RegistryClient::get_records`] that caches the result.
    ///
    /// It takes [`ManifestDependency`] instead of [`PackageName`] to allow performing some
    /// optimizations by pre-filtering index records on cache-level.
    #[tracing::instrument(level = "trace", skip_all)]
    pub async fn get_records_with_cache(
        &self,
        dependency: &ManifestDependency,
    ) -> Result<IndexRecords> {
        let package_name = dependency.name.as_str();
        let db = self.db().await?;

        let cache_key = db.get_records_cache_key(package_name).await?;

        match self
            .client
            .get_records(dependency.name.clone(), cache_key.as_deref())
            .await?
        {
            RegistryResource::NotFound => {
                db.prune_records(package_name).await?;
                bail!("package not found in registry: {dependency}")
            }

            RegistryResource::InCache => db.get_records(dependency).await,

            RegistryResource::Download {
                resource: records,
                cache_key,
            } => {
                if let Some(cache_key) = cache_key {
                    db.upsert_records(package_name, cache_key, &records).await?;
                }
                Ok(records)
            }
        }
    }

    /// Layer over [`RegistryClient::download`] that caches the result.
    #[tracing::instrument(level = "trace", skip_all)]
    pub async fn download_with_cache(&self, package: PackageId) -> Result<PathBuf> {
        match self.client.download(package).await? {
            RegistryResource::NotFound => {
                trace!("archive not found in registry, pruning cache");
                bail!("could not find downloadable archive for package indexed in registry: {package}")
            }
            RegistryResource::InCache => {
                trace!("using cached archive");
                todo!()
            }
            RegistryResource::Download { resource, .. } => {
                trace!("got new archive, invalidating cache");
                Ok(resource)
            }
        }
    }

    async fn db(&self) -> Result<&CacheDatabase> {
        self.db_cell
            .get_or_try_init(|| async {
                let ui = self.config.ui();
                let fs = self.config.dirs().registry_dir().into_child("cache");
                let db_path = fs
                    .path_existent()?
                    .join(format!("{}.v1.redb", self.source_id.ident()));

                CacheDatabase::create(&db_path, ui)
            })
            .await
    }
}

struct CacheDatabase {
    db: redb::Database,
    ui: Ui,
}

impl CacheDatabase {
    #[tracing::instrument(level = "trace", skip(ui))]
    fn create(path: &Utf8Path, ui: Ui) -> Result<Self> {
        fn create(path: &Utf8Path, ui: &Ui) -> Result<redb::Database> {
            redb::Database::create(path)
                .context("failed to open local registry cache, trying to recreate it")
                .or_else(|error| {
                    ui.warn_anyhow(&error);
                    fsx::remove_file(path).context("failed to remove local registry cache")?;
                    redb::Database::create(path)
                        .with_context(|| db_fatal("failed to open local registry cache"))
                })
        }

        fn init_tables(db: &redb::Database) -> Result<()> {
            let tx = db.begin_write()?;
            {
                tx.open_multimap_table(RECORDS)?;
                tx.open_table(RECORDS_CACHE_KEYS)?;
            }
            tx.commit()?;
            Ok(())
        }

        trace!("opening local registry cache: {path}");
        let db = block_in_place(|| -> Result<_> {
            let db = create(path, &ui)?;
            trace!("database opened/created successfully");
            init_tables(&db).context("failed to initialize local registry cache database")?;
            trace!("created all tables in local registry cache database");
            Ok(db)
        })?;

        Ok(Self { db, ui })
    }

    #[tracing::instrument(level = "trace", skip_all)]
    async fn get_records_cache_key(&self, package_name: &str) -> Result<Option<String>> {
        trace!("looking up cache key");
        block_in_place(|| -> Result<_> {
            let tx = self.db.begin_read()?;
            let table = tx.open_table(RECORDS_CACHE_KEYS)?;
            let cache_key = table.get(package_name)?.map(|g| g.value().to_owned());
            trace!(?cache_key);
            Ok(cache_key)
        })
        .with_context(|| db_error("failed to lookup cache key in registry cache"))
        .or_else(|err| -> Result<_> {
            self.ui.warn_anyhow(&err);
            Ok(None)
        })
    }

    #[tracing::instrument(level = "trace", skip_all)]
    async fn get_records(&self, dependency: &ManifestDependency) -> Result<IndexRecords> {
        trace!("getting records from cache");
        block_in_place(|| -> Result<_> {
            let tx = self.db.begin_read()?;
            let table = tx.open_multimap_table(RECORDS)?;

            let mut records = IndexRecords::new();
            for g in table.get(dependency.name.as_str())? {
                let g = g?;
                let (raw_version, raw_record) = g.value();

                let version = Version::parse(raw_version)
                    .with_context(|| db_fatal("failed to parse version from cache"))?;
                if !dependency.matches_name_and_version(&dependency.name, &version) {
                    continue;
                }

                let record = serde_json::from_slice::<IndexRecord>(raw_record)
                    .with_context(|| db_fatal("failed to deserialize index record from cache"))?;

                records.push(record);
            }
            trace!("records read successfully");
            Ok(records)
        })
    }

    #[tracing::instrument(level = "trace", skip_all)]
    async fn prune_records(&self, package_name: &str) -> Result<()> {
        trace!("package not found in registry, pruning cache");
        block_in_place(|| -> Result<_> {
            let tx = self.db.begin_write()?;
            {
                let mut table = tx.open_multimap_table(RECORDS)?;
                table.remove_all(package_name)?;
            }
            tx.commit()?;
            trace!("cache pruned successfully");
            Ok(())
        })
        .with_context(|| db_error("failed to purge cache from now non-existent entries"))
        .or_else(|err| -> Result<_> {
            self.ui.warn_anyhow(&err);
            Ok(())
        })?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip_all)]
    async fn upsert_records(
        &self,
        package_name: &str,
        cache_key: String,
        records: &IndexRecords,
    ) -> Result<()> {
        trace!("got new records, invalidating cache");
        trace!(?cache_key);
        block_in_place(|| -> Result<_> {
            let tx = self.db.begin_write()?;
            {
                let mut table = tx.open_table(RECORDS_CACHE_KEYS)?;
                table.insert(package_name, cache_key.as_str())?;
            }
            {
                let mut table = tx.open_multimap_table(RECORDS)?;
                table.remove_all(package_name)?;

                for record in records {
                    let raw_version = record.version.to_string();
                    let raw_record = serde_json::to_vec(&record)?;
                    table.insert(package_name, (raw_version.as_str(), raw_record.as_slice()))?;
                }
            }
            tx.commit()?;
            trace!("cache updated successfully");
            Ok(())
        })
        .with_context(|| db_error("failed to cache registry index records"))
        .or_else(|err| -> Result<_> {
            self.ui.warn_anyhow(&err);
            Ok(())
        })
    }
}

fn db_error(message: &str) -> String {
    format!(
        "{message}\n\
        note: perhaps cache is corrupted\n\
        help: try restarting scarb to recreate it"
    )
}

fn db_fatal(message: &str) -> String {
    format!(
        "{message}\n\
        note: cache is corrupted and is in unrecoverable state\n\
        help: run the following to wipe entire cache: scarb cache clean"
    )
}
