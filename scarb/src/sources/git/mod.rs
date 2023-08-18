use std::{fmt, mem};

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::sync::OnceCell;
use tokio::task::spawn_blocking;
use url::Url;

use canonical_url::CanonicalUrl;
use client::{GitRemote, Rev};
use scarb_ui::components::Status;

use crate::core::source::Source;
use crate::core::{
    Config, GitReference, ManifestDependency, Package, PackageId, SourceId, Summary,
};
use crate::sources::git::client::GitDatabase;

use super::PathSource;

pub mod canonical_url;
mod client;

pub struct GitSource<'c> {
    source_id: SourceId,
    config: &'c Config,
    remote: GitRemote,
    requested_reference: GitReference,
    locked_rev: Option<Rev>,
    inner: OnceCell<InnerState<'c>>,
}

struct InnerState<'c> {
    path_source: PathSource<'c>,
    // TODO(#126): Update rev lock in the lockfile with this value.
    actual_rev: Rev,
}

impl<'c> GitSource<'c> {
    pub fn new(source_id: SourceId, config: &'c Config) -> Result<Self> {
        Self::with_custom_repo(
            &source_id.url,
            source_id.git_reference().unwrap(),
            source_id,
            config,
        )
    }

    pub fn with_custom_repo(
        repo_url: &Url,
        requested_reference: GitReference,
        source_id: SourceId,
        config: &'c Config,
    ) -> Result<Self> {
        let canonical_url = CanonicalUrl::new(repo_url)?;

        Ok(Self {
            source_id,
            config,
            remote: GitRemote::new(canonical_url),
            requested_reference,
            // TODO(#126): Pull this somehow from the lockfile.
            locked_rev: None,
            inner: OnceCell::new(),
        })
    }

    async fn ensure_loaded(&self) -> Result<&InnerState<'c>> {
        self.inner.get_or_try_init(|| self.load()).await
    }

    async fn load(&self) -> Result<InnerState<'c>> {
        let _lock = self.config.package_cache_lock().acquire_async().await?;

        let source_id = self.source_id;
        let remote = self.remote.clone();
        let requested_reference = self.requested_reference.clone();
        let locked_rev = self.locked_rev;

        // HACK: We know that we will not use &Config outside scope of this function,
        //   but `smol::unblock` lifetime bounds force us to think so.
        let config: &'static Config = unsafe { mem::transmute(self.config) };

        return spawn_blocking(move || {
            inner(source_id, remote, requested_reference, locked_rev, config)
        })
        .await?;

        fn inner(
            source_id: SourceId,
            remote: GitRemote,
            requested_reference: GitReference,
            locked_rev: Option<Rev>,
            config: &Config,
        ) -> Result<InnerState<'_>> {
            let remote_ident = remote.ident();

            let registry_fs = config.dirs().registry_dir();
            let git_fs = registry_fs.child("git");
            let all_db_fs = git_fs.child("db");

            let db_fs = all_db_fs.child(&format!("{remote_ident}.git"));
            let db = GitDatabase::open(&remote, &db_fs).ok();
            let (db, actual_rev) = match (db, locked_rev) {
                // If we have a locked revision, and we have a preexisting database
                // which has that revision, then no update needs to happen.
                (Some(db), Some(rev)) if db.contains(rev) => (db, rev),

                // If Scarb is in offline mode, source is not locked to particular revision,
                // and there is a functional database, then try to resolve our reference
                // with the preexisting repository.
                (Some(db), None) if !config.network_allowed() => {
                    let rev = db.resolve(&requested_reference).context(
                        "failed to lookup reference in preexisting repository, and \
                        cannot check for updates in offline mode (--offline)",
                    )?;
                    (db, rev)
                }

                // Now we can freely update the database.
                (db, locked_rev) => {
                    // The actual error will be produced by `checkout`.
                    if config.network_allowed() {
                        config
                            .ui()
                            .print(Status::new("Updating", &format!("git repository {remote}")));
                    }

                    remote.checkout(&db_fs, db, &requested_reference, locked_rev, config)?
                }
            };

            let all_checkouts_fs = git_fs.child("checkouts");
            let db_checkouts_fs = all_checkouts_fs.child(&remote_ident);
            let checkout_fs = db_checkouts_fs.child(db.short_id_of(actual_rev)?);
            let checkout = db.copy_to(&checkout_fs, actual_rev, config)?;

            let path_source = PathSource::recursive_at(&checkout.location, source_id, config);

            Ok(InnerState {
                path_source,
                actual_rev,
            })
        }
    }
}

#[async_trait]
impl<'c> Source for GitSource<'c> {
    async fn query(&self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
        self.ensure_loaded()
            .await?
            .path_source
            .query(dependency)
            .await
    }

    async fn download(&self, id: PackageId) -> Result<Package> {
        self.ensure_loaded().await?.path_source.download(id).await
    }
}

impl<'c> fmt::Debug for GitSource<'c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GitSource")
            .field("source", &self.source_id.to_string())
            .field("remote", &self.remote)
            .field("requested_reference", &self.requested_reference)
            .field("locked_rev", &self.locked_rev)
            .field("actual_rev", &self.inner.get().map(|s| &s.actual_rev))
            .finish_non_exhaustive()
    }
}
