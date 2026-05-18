use anyhow::{Context, Result};

use crate::core::Config;

// Name of the package cache lock file inside the cache dir. Kept in sync with
// `Config::package_cache_lock`. `cache_clean` must NOT unlink this file: the
// `flock` held by the in-progress clean is per-inode, and unlinking allows a
// concurrent `acquire_async` to create a different inode at the same path
// and succeed against it as if uncontended — both processes would then
// believe they hold the exclusive lock at the same time. The path is also
// part of the on-disk contract with older scarb versions, so we cannot move
// it without losing cross-version mutual exclusion.
const PACKAGE_CACHE_LOCK_FILENAME: &str = ".package-cache.lock";

#[tracing::instrument(skip_all, level = "debug")]
pub fn cache_clean(config: &Config) -> Result<()> {
    let path = config.dirs().cache_dir.path_unchecked();
    if path.exists() {
        let _lock = config
            .tokio_handle()
            .block_on(config.package_cache_lock().acquire_async())?;

        // Remove every entry in the cache dir except the lock file itself.
        // See `PACKAGE_CACHE_LOCK_FILENAME` for why the lock file must stay.
        for entry in std::fs::read_dir(path).context("failed to clean cache")? {
            let entry = entry.context("failed to clean cache")?;
            if entry.file_name() == PACKAGE_CACHE_LOCK_FILENAME {
                continue;
            }
            let entry_path = entry.path();
            let file_type = entry
                .file_type()
                .with_context(|| format!("failed to stat cache entry: {}", entry_path.display()))?;
            if file_type.is_dir() {
                scarb_fs_utils::remove_dir_all(&entry_path).with_context(|| {
                    format!("failed to clean cache entry: {}", entry_path.display())
                })?;
            } else {
                std::fs::remove_file(&entry_path).with_context(|| {
                    format!("failed to clean cache entry: {}", entry_path.display())
                })?;
            }
        }

        test_pause_hook();
    }
    Ok(())
}

// Test-only synchronization point (see
// `cache_clean_race_with_concurrent_lock_acquisition` in `scarb/tests/cache.rs`).
//
// When `SCARB_INTERNAL_CACHE_CLEAN_PAUSE` is set to a path prefix `P`, this:
//   1. creates `P.ready` to signal the test that the lock is held and the
//      cache contents (other than the lock file) have just been removed,
//   2. blocks until `P.go` appears.
//
// At this point the cache dir has been emptied except for the lock file, and
// `_lock` in `cache_clean` still holds the `flock` — exactly the state a
// concurrent `acquire_async` must observe and block on. The env var is
// namespaced as `_INTERNAL_` to make clear it is not part of the public
// CLI/env contract.
#[cfg(feature = "test-utils")]
fn test_pause_hook() {
    let Ok(prefix) = std::env::var("SCARB_INTERNAL_CACHE_CLEAN_PAUSE") else {
        return;
    };
    let _ = std::fs::write(format!("{prefix}.ready"), "");
    let go = std::path::PathBuf::from(format!("{prefix}.go"));
    while !go.exists() {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[cfg(not(feature = "test-utils"))]
fn test_pause_hook() {}
