use std::fs::{File, OpenOptions};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Weak};
use std::{fmt, io};

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use fs4::{lock_contended_error, FileExt};
use tokio::sync::Mutex;

use scarb_ui::components::Status;

use crate::core::Config;
use crate::internal::fsx;
use crate::internal::lazy_directory_creator::LazyDirectoryCreator;

const OK_FILE: &str = ".scarb-ok";

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FileLockKind {
    Shared,
    Exclusive,
}

#[derive(Debug)]
pub struct FileLockGuard {
    file: Option<File>,
    path: Utf8PathBuf,
    lock_kind: FileLockKind,
}

impl FileLockGuard {
    pub fn path(&self) -> &Utf8Path {
        self.path.as_path()
    }

    pub fn lock_kind(&self) -> FileLockKind {
        self.lock_kind
    }
}

impl Deref for FileLockGuard {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        self.file.as_ref().unwrap()
    }
}

impl DerefMut for FileLockGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.file.as_mut().unwrap()
    }
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = file.unlock();
        }
    }
}

/// An exclusive lock over a global entity identified by a path within a [`Filesystem`].
pub struct AdvisoryLock<'f> {
    path: Utf8PathBuf,
    description: String,
    file_lock: Mutex<
        // This Arc is shared between all guards within the process.
        // Here it is Weak, because AdvisoryLock itself does not keep the lock
        // (only guards do).
        Weak<FileLockGuard>,
    >,
    filesystem: &'f Filesystem<'f>,
    config: &'f Config,
}

#[derive(Debug)]
pub struct AdvisoryLockGuard(Arc<FileLockGuard>);

impl<'f> AdvisoryLock<'f> {
    /// Acquires this advisory lock in an async manner.
    ///
    /// This lock is global per-process and can be acquired recursively.
    /// An RAII structure is returned to release the lock, and if this process abnormally
    /// terminates the lock is also released.
    pub async fn acquire_async(&self) -> Result<AdvisoryLockGuard> {
        let mut slot = self.file_lock.lock().await;

        let file_lock_arc = match slot.upgrade() {
            Some(arc) => arc,
            None => {
                let arc = Arc::new(self.filesystem.open_rw(
                    &self.path,
                    &self.description,
                    self.config,
                )?);
                *slot = Arc::downgrade(&arc);
                arc
            }
        };
        Ok(AdvisoryLockGuard(file_lock_arc))
    }
}

/// A [`Filesystem`] that does not have a parent.
pub type RootFilesystem = Filesystem<'static>;

/// A [`Filesystem`] is intended to be a globally shared, hence locked, resource in Scarb.
///
/// The [`Utf8Path`] of a file system cannot be learned unless it's done in a locked fashion,
/// and otherwise functions on this structure are prepared to handle concurrent invocations across
/// multiple instances of Scarb and its extensions.
///
/// All paths within a [`Filesystem`] must be UTF-8 encoded.
pub struct Filesystem<'a> {
    root: LazyDirectoryCreator<'a>,
}

impl<'a> Filesystem<'a> {
    /// Creates a new [`Filesystem`] to be rooted at the given path.
    pub fn new(root: Utf8PathBuf) -> Self {
        Self {
            root: LazyDirectoryCreator::new(root),
        }
    }

    /// Creates a new [`Filesystem`] to be rooted at the given path.
    ///
    /// This variant uses [`create_output_dir::create_output_dir`] function to create root
    /// directory.
    pub fn new_output_dir(root: Utf8PathBuf) -> Self {
        Self {
            root: LazyDirectoryCreator::new_output_dir(root),
        }
    }

    /// Like [`Utf8Path::join`], creates a new [`Filesystem`] rooted at a subdirectory of this one.
    pub fn child(&self, path: impl AsRef<Utf8Path>) -> Filesystem<'_> {
        Filesystem {
            root: self.root.child(path),
        }
    }

    /// Get path to this [`Filesystem`] root without ensuring the path exists.
    pub fn path_unchecked(&self) -> &Utf8Path {
        self.root.as_unchecked()
    }

    /// Get path to this [`Filesystem`] root, ensuring the path exists.
    pub fn path_existent(&self) -> Result<&Utf8Path> {
        self.root.as_existent()
    }

    /// Opens exclusive access to a [`File`], returning the locked version of it.
    ///
    /// This function will create a file at `path` if it doesn't already exist (including
    /// intermediate directories) else if it does exist, it will be truncated. It will then acquire
    /// an exclusive lock on `path`. If the process must block waiting for the lock, the
    /// `description` annotated with _blocking_ status message is printed to [`Config::ui`].
    ///
    /// The returned file can be accessed to look at the path and also has read/write access to
    /// the underlying file.
    pub fn open_rw(
        &self,
        path: impl AsRef<Utf8Path>,
        description: &str,
        config: &Config,
    ) -> Result<FileLockGuard> {
        self.open(
            path.as_ref(),
            OpenOptions::new()
                .read(true)
                .write(true)
                .truncate(true)
                .create(true),
            FileLockKind::Exclusive,
            description,
            config,
        )
    }

    /// Opens shared access to a [`File`], returning the locked version of it.
    ///
    /// This function will fail if `path` doesn't already exist, but if it does then it will
    /// acquire a shared lock on `path`.
    /// If the process must block waiting for the lock, the `description` annotated with _blocking_
    /// status message is printed to [`Config::ui`].
    ///
    /// The returned file can be accessed to look at the path and also has read
    /// access to the underlying file.
    /// Any writes to the file will return an error.
    pub fn open_ro(
        &self,
        path: impl AsRef<Utf8Path>,
        description: &str,
        config: &Config,
    ) -> Result<FileLockGuard> {
        self.open(
            path.as_ref(),
            OpenOptions::new().read(true),
            FileLockKind::Shared,
            description,
            config,
        )
    }

    fn open(
        &self,
        path: &Utf8Path,
        opts: &OpenOptions,
        lock_kind: FileLockKind,
        description: &str,
        config: &Config,
    ) -> Result<FileLockGuard> {
        let path = self.root.as_existent()?.join(path);

        let file = opts
            .open(&path)
            .with_context(|| format!("failed to open: {path}"))?;

        match lock_kind {
            FileLockKind::Exclusive => {
                acquire(
                    &file,
                    &path,
                    description,
                    config,
                    &FileExt::try_lock_exclusive,
                    &FileExt::lock_exclusive,
                )?;
            }
            FileLockKind::Shared => {
                acquire(
                    &file,
                    &path,
                    description,
                    config,
                    &FileExt::try_lock_shared,
                    &FileExt::lock_shared,
                )?;
            }
        }

        Ok(FileLockGuard {
            file: Some(file),
            path,
            lock_kind,
        })
    }

    /// Construct an [`AdvisoryLock`] within this file system.
    pub fn advisory_lock(
        &'a self,
        path: impl AsRef<Utf8Path>,
        description: impl ToString,
        config: &'a Config,
    ) -> AdvisoryLock<'a> {
        AdvisoryLock {
            path: path.as_ref().to_path_buf(),
            description: description.to_string(),
            file_lock: Mutex::new(Weak::new()),
            filesystem: self,
            config,
        }
    }

    /// Remove the directory underlying this filesystem and create it again.
    ///
    /// # Safety
    /// This is very simple internal method meant to be used in very specific use-cases, so its
    /// implementation does not handle all cases.
    /// 1. Panics if this is an output filesystem.
    /// 2. Child filesystems will stop working properly after recreation.
    pub(crate) unsafe fn recreate(&self) -> Result<()> {
        if self.root.is_output_dir() {
            panic!("cannot recreate output filesystems");
        }

        let path = self.root.as_unchecked();
        if path.exists() {
            fsx::remove_dir_all(path)?;
        }
        fsx::create_dir_all(path)?;
        Ok(())
    }

    /// Checks if this filesystem has a valid `.scarb-ok` file.
    pub fn is_ok(&self) -> bool {
        self.path_unchecked().join(OK_FILE).exists()
    }

    /// Marks this filesystem as being properly set up (whatever this means is up to user),
    /// by creating a `.scarb-ok` file.
    pub fn mark_ok(&self) -> Result<()> {
        let _ = fsx::create(self.path_existent()?.join(OK_FILE))?;
        Ok(())
    }
}

impl<'a> fmt::Display for Filesystem<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.root)
    }
}

impl<'a> fmt::Debug for Filesystem<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Filesystem").field(&self.root).finish()
    }
}

fn acquire(
    file: &File,
    path: &Utf8Path,
    description: &str,
    config: &Config,
    lock_try: &dyn Fn(&File) -> io::Result<()>,
    lock_block: &dyn Fn(&File) -> io::Result<()>,
) -> Result<()> {
    match lock_try(file) {
        Ok(()) => return Ok(()),
        Err(err) if err.kind() == io::ErrorKind::Unsupported => {
            // Ignore locking on filesystems that look like they don't implement file locking.
            return Ok(());
        }
        Err(err) if is_lock_contended_error(&err) => {
            // Pass-through
        }
        Err(err) => {
            Err(err).with_context(|| format!("failed to lock file: {path}"))?;
        }
    }

    config.ui().print(Status::with_color(
        "Blocking",
        "cyan",
        &format!("waiting for file lock on {description}"),
    ));

    lock_block(file).with_context(|| format!("failed to lock file: {path}"))?;

    Ok(())
}

fn is_lock_contended_error(err: &io::Error) -> bool {
    let t = lock_contended_error();
    err.raw_os_error() == t.raw_os_error() || err.kind() == t.kind()
}
