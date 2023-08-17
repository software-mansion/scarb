//! High-level wrapper over Scarb Git functionality.
//!
//! Most of the code here, including relevant comments, has been copied with slight modifications
//! from Cargo. The primary modifications are:
//! 1. Usage of `gitoxide` instead of `libgit2`.
//! 2. Fetches and clones are always delegated to Git CLI.
//! 3. There is no special GitHub fast-path, because in long-term we do not want to treat Git
//!    repositories as source of super important information.

use std::fmt;
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use camino::Utf8PathBuf;

use scarb_ui::Verbosity;

use crate::core::{Config, GitReference};
use crate::flock::Filesystem;
use crate::process::exec;

use super::canonical_url::CanonicalUrl;

/// A Git remote repository that can be cloned into a local [`GitDatabase`].
#[derive(Clone, Eq, PartialEq)]
pub struct GitRemote {
    url: CanonicalUrl,
}

impl fmt::Display for GitRemote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.url)
    }
}

impl fmt::Debug for GitRemote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GitRemote({})", self.url)
    }
}

/// A local clone of a remote ([`GitRemote`]) Git repository's database.
///
/// Multiple [`GitCheckout`]s can be cloned from this database.
pub struct GitDatabase {
    remote: GitRemote,
    path: Utf8PathBuf,
    repo: gix::Repository,
}

impl fmt::Debug for GitDatabase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GitDatabase")
            .field("remote", &self.remote)
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

/// A local checkout of a particular Git commit.
#[derive(Debug)]
pub struct GitCheckout<'d> {
    pub db: &'d GitDatabase,
    pub location: Utf8PathBuf,
    pub rev: Rev,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Rev {
    oid: gix::ObjectId,
}

impl fmt::Display for Rev {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.oid)
    }
}

impl fmt::Debug for Rev {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rev({self})")
    }
}

impl From<gix::ObjectId> for Rev {
    fn from(oid: gix::ObjectId) -> Self {
        Self { oid }
    }
}

impl GitRemote {
    pub fn new(url: CanonicalUrl) -> Self {
        Self { url }
    }

    pub fn ident(&self) -> String {
        self.url.ident()
    }

    #[tracing::instrument(level = "trace", skip(config))]
    pub fn checkout(
        &self,
        fs: &Filesystem<'_>,
        db: Option<GitDatabase>,
        reference: &GitReference,
        locked_rev: Option<Rev>,
        config: &Config,
    ) -> Result<(GitDatabase, Rev)> {
        // If we have a previous instance of `GitDatabase` then fetch into that if we can.
        // If that can successfully load our revision then we've populated the database with the latest
        // version of `reference`, so return that database and the rev we resolve to.
        if let Some(db) = db {
            db.fetch(self.url.as_str(), reference, config)
                .with_context(|| format!("failed to fetch into: {fs}"))?;
            match locked_rev {
                Some(rev) => {
                    if db.contains(rev) {
                        return Ok((db, rev));
                    }
                }
                None => {
                    if let Ok(rev) = db.resolve(reference) {
                        return Ok((db, rev));
                    }
                }
            }
        }

        // Otherwise start from scratch to handle corrupt git repositories.
        // After our fetch (which is interpreted as a clone now) we do the same
        // resolution to figure out what we cloned.
        unsafe {
            fs.recreate()?;
        }
        let db = GitDatabase::init_bare(self, fs)?;
        db.fetch(self.url.as_str(), reference, config)
            .with_context(|| format!("failed to clone into: {fs}"))?;
        let rev = match locked_rev {
            Some(rev) => rev,
            None => db.resolve(reference)?,
        };
        Ok((db, rev))
    }
}

impl GitDatabase {
    #[tracing::instrument(level = "trace")]
    pub fn open(remote: &GitRemote, fs: &Filesystem<'_>) -> Result<Self> {
        let path = fs.path_existent()?;
        let opts = gix::open::Options::default().open_path_as_is(true);
        let repo = gix::open_opts(path, opts)?;
        Ok(Self {
            remote: remote.clone(),
            path: path.to_path_buf(),
            repo,
        })
    }

    #[tracing::instrument(level = "trace")]
    pub fn init_bare(remote: &GitRemote, fs: &Filesystem<'_>) -> Result<Self> {
        let path = fs.path_existent()?;
        let repo = gix::init_bare(path)?;
        Ok(Self {
            remote: remote.clone(),
            path: path.to_path_buf(),
            repo,
        })
    }

    #[tracing::instrument(level = "trace", skip(config))]
    fn fetch(&self, url: &str, reference: &GitReference, config: &Config) -> Result<()> {
        if !config.network_allowed() {
            bail!("cannot fetch from `{}` in offline mode", self.remote);
        }

        let (refspecs, fetch_tags) = collect_refspecs(reference);

        let mut cmd = git_command();
        cmd.arg("fetch");
        if fetch_tags {
            cmd.arg("--tags");
        }
        with_verbosity_flags(&mut cmd, config);
        // Handle force pushes.
        cmd.arg("--force");
        // https://stackoverflow.com/questions/2236743/git-refusing-to-fetch-into-current-branch
        cmd.arg("--update-head-ok");
        cmd.arg(url);
        cmd.args(refspecs);
        cmd.current_dir(self.repo.path());
        exec(&mut cmd, config)
    }

    pub fn copy_to(
        &self,
        fs: &Filesystem<'_>,
        rev: Rev,
        config: &Config,
    ) -> Result<GitCheckout<'_>> {
        let checkout = GitCheckout::clone(self, fs, rev, config)?;
        checkout.reset(config)?;
        Ok(checkout)
    }

    pub fn contains(&self, rev: Rev) -> bool {
        self.repo.rev_parse_single(rev.oid.as_bytes()).is_ok()
    }

    #[tracing::instrument(level = "trace")]
    pub fn resolve(&self, reference: &GitReference) -> Result<Rev> {
        use GitReference::*;
        let repo = &self.repo;
        match reference {
            Tag(t) => Ok(repo
                .try_find_reference(&format!("refs/remotes/origin/tags/{t}"))
                .with_context(|| format!("failed to find tag `{t}`"))?
                .ok_or_else(|| anyhow!("tag `{t}` does not exist"))?
                .peel_to_id_in_place()
                .with_context(|| format!("tag `{t}` does not have a target"))?
                .detach()
                .into()),

            Branch(b) => Ok(repo
                .try_find_reference(&format!("origin/{b}"))
                .with_context(|| format!("failed to find branch `{b}`"))?
                .ok_or_else(|| anyhow!("branch `{b}` does not exist"))?
                .peel_to_id_in_place()
                .with_context(|| format!("branch `{b}` does not have a target"))?
                .detach()
                .into()),

            Rev(rev) => Ok(repo
                .rev_parse_single(rev.as_str())?
                .object()?
                .peel_tags_to_end()?
                .id
                .into()),

            DefaultBranch => Ok(repo
                .find_reference("refs/remotes/origin/HEAD")?
                .peel_to_id_in_place()?
                .detach()
                .into()),
        }
    }

    pub fn short_id_of(&self, rev: Rev) -> Result<String> {
        let obj = self.repo.find_object(rev.oid)?;
        Ok(obj.id().shorten_or_id().to_string())
    }
}

impl<'d> GitCheckout<'d> {
    #[tracing::instrument(level = "trace", skip(config))]
    fn clone(db: &'d GitDatabase, fs: &Filesystem<'_>, rev: Rev, config: &Config) -> Result<Self> {
        unsafe {
            fs.recreate()?;
        }

        let location = fs.path_existent()?.to_path_buf();

        let mut cmd = git_command();
        cmd.args(["clone", "--local"]);
        with_verbosity_flags(&mut cmd, config);
        cmd.args(["--config", "core.autocrlf=false"]);
        cmd.arg("--recurse-submodules");
        cmd.arg(db.repo.path());
        cmd.arg(&location);
        exec(&mut cmd, config)?;

        Ok(Self { db, location, rev })
    }

    #[tracing::instrument(level = "trace", skip(config))]
    fn reset(&self, config: &Config) -> Result<()> {
        let mut cmd = git_command();
        cmd.args(["reset", "--hard"]);
        cmd.arg(self.rev.to_string());
        cmd.current_dir(&self.location);
        exec(&mut cmd, config)
    }
}

/// Translate the [`GitReference`] into an actual list of Git _refspecs_ which need to be fetched.
///
/// Additionally, this function records if there is a need to fetch tags.
///
/// The `+` symbol on the _refspec_ means to allow a forced (fast-forward) update which is needed
/// if there is ever a force push that requires a fast-forward.
fn collect_refspecs(reference: &GitReference) -> (Vec<String>, bool) {
    use GitReference::*;

    match reference {
        // For branches and tags we can simply fetch one reference and copy it locally,
        // no need to fetch other branches/tags.
        Branch(b) => (
            vec![format!("+refs/heads/{0}:refs/remotes/origin/{0}", b)],
            false,
        ),
        Tag(t) => (
            vec![format!("+refs/tags/{0}:refs/remotes/origin/tags/{0}", t)],
            false,
        ),

        DefaultBranch => (vec!["+HEAD:refs/remotes/origin/HEAD".to_string()], false),

        Rev(rev) if rev.starts_with("refs/") => (vec![format!("+{0}:{0}", rev)], false),

        Rev(_) => (
            // We don't know what the rev will point to.
            // To handle this situation we fetch all branches and tags,
            // and then we pray it's somewhere in there.
            vec![
                "+refs/heads/*:refs/remotes/origin/*".to_string(),
                "+HEAD:refs/remotes/origin/HEAD".to_string(),
            ],
            true,
        ),
    }
}

fn git_command() -> Command {
    let mut cmd = Command::new("git");

    // If Scarb is run by Git (for example, the `exec` command in `git rebase`),
    // the GIT_DIR is set by Git and will point to the wrong location (this takes precedence
    // over the cwd). Make sure this is unset so git will look at cwd for the repo.
    cmd.env_remove("GIT_DIR");
    // Cargo does this just to be extra paranoid, so do we.
    cmd.env_remove("GIT_WORK_TREE");
    cmd.env_remove("GIT_INDEX_FILE");
    cmd.env_remove("GIT_OBJECT_DIRECTORY");
    cmd.env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES");

    cmd
}

fn with_verbosity_flags(cmd: &mut Command, config: &Config) {
    match config.ui().verbosity() {
        Verbosity::Normal => {}
        Verbosity::Verbose => {
            cmd.arg("--verbose");
        }
        Verbosity::Quiet => {
            cmd.arg("--quiet");
        }
    }
}
