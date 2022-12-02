use std::path::Path;

use anyhow::Result;

// TODO(mkaput): Copy-paste implementation here so that we don't pull and compile unnecessary
//   stuff from the `cargo_util` crate.
/// Creates an excluded from cache directory atomically with its parents as needed.
///
/// The atomicity only covers creating the leaf directory and exclusion from cache. Any missing
/// parent directories will not be created in an atomic manner.
///
/// This function is idempotent and in addition to that it won't exclude `path` from cache if it
/// already exists.
pub fn create_output_dir(path: &Path) -> Result<()> {
    cargo_util::paths::create_dir_all_excluded_from_backups_atomic(path)
}
