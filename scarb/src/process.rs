use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

use crate::core::Config;
use crate::ui::Status;

// TODO(mkaput): Do what is documented here, take a look at what cargo-util does.
/// Replaces the current process with the target process.
///
/// On Unix, this executes the process using the Unix syscall `execvp`, which will block this
/// process, and will only return if there is an error.
///
/// On Windows this isn't technically possible. Instead we emulate it to the best of our ability.
/// One aspect we fix here is that we specify a handler for the Ctrl-C handler.
/// In doing so (and by effectively ignoring it) we should emulate proxying Ctrl-C handling to
/// the application at hand, which will either terminate or handle it itself.
/// According to Microsoft's documentation at
/// https://docs.microsoft.com/en-us/windows/console/ctrl-c-and-ctrl-break-signals.
/// the Ctrl-C signal is sent to all processes attached to a terminal, which should include our
/// child process. If the child terminates then we'll reap them in Cargo pretty quickly, and if
/// the child handles the signal then we won't terminate (and we shouldn't!) until the process
/// itself later exits.
#[tracing::instrument(level = "debug")]
pub fn exec_replace(cmd: &mut Command) -> Result<()> {
    let exit_status = cmd
        .spawn()
        .with_context(|| format!("failed to spawn: {}", cmd.get_program().to_string_lossy()))?
        .wait()
        .with_context(|| {
            format!(
                "failed to wait for process to finish: {}",
                cmd.get_program().to_string_lossy()
            )
        })?;

    if exit_status.success() {
        Ok(())
    } else {
        bail!("process did not exit successfully: {exit_status}");
    }
}

// TODO(mkaput): Capture stdout/stderr in intelligent way.
/// Runs the process, waiting for completion, and mapping non-success exit codes to an error.
#[tracing::instrument(level = "debug", skip(config))]
pub fn exec(cmd: &mut Command, config: &Config) -> Result<()> {
    config
        .ui()
        .verbose(Status::new("Running", &format!("{cmd:?}")));

    let output = cmd
        .output()
        .with_context(|| anyhow!("could not execute process {cmd:?}"))?;

    if output.status.success() {
        Ok(())
    } else {
        bail!("process did not exit successfully: {}", output.status);
    }
}

#[cfg(unix)]
pub fn is_executable<P: AsRef<Path>>(path: P) -> bool {
    use std::fs;
    use std::os::unix::prelude::*;
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(windows)]
pub fn is_executable<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().is_file()
}
