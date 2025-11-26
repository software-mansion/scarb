//! Helper utilities shared by Scarb execute-related commands.

use anyhow::{Result, bail};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use std::io;

/// The maximum number of attempts when looking for a free execution directory name.
const MAX_ITERATION_COUNT: usize = 10000;

/// File capturing the program output (stderr) produced by `scarb execute`.
pub const EXECUTE_PROGRAM_OUTPUT_FILENAME: &str = "program_output.txt";
/// File capturing the standard output produced by `scarb execute`.
pub const EXECUTE_PRINT_OUTPUT_FILENAME: &str = "stdout_output.txt";

/// Creates an incremental execution output dir inside the given path.
/// The dir name is `execution{N}` with the lowest `N` without existing dir.
///
/// Returns the path to the created directory and corresponding `N`.
pub fn incremental_create_execution_output_dir(path: &Utf8Path) -> Result<(Utf8PathBuf, usize)> {
    for i in 1..=MAX_ITERATION_COUNT {
        let filepath = path.join(format!("execution{i}"));
        match fs::create_dir(&filepath) {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
            Ok(_) => return Ok((filepath, i)),
        }
    }
    bail!("failed to create output directory")
}
