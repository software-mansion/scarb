use std::ffi::OsStr;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::{fmt, thread};

use anyhow::{anyhow, bail, Context, Result};
use tracing::{debug, debug_span, warn, Span};

use scarb_ui::components::{Spinner, Status};

use crate::core::Config;
pub use crate::internal::fsx::is_executable;

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
/// <https://docs.microsoft.com/en-us/windows/console/ctrl-c-and-ctrl-break-signals>.
/// the Ctrl-C signal is sent to all processes attached to a terminal, which should include our
/// child process. If the child terminates then we'll reap them in Cargo pretty quickly, and if
/// the child handles the signal then we won't terminate (and we shouldn't!) until the process
/// itself later exits.
#[tracing::instrument(level = "debug", skip_all)]
pub fn exec_replace(cmd: &mut Command) -> Result<()> {
    debug!("{}", shlex_join(cmd));
    imp::exec_replace(cmd)
}

#[cfg(unix)]
mod imp {
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    use anyhow::{bail, Result};

    pub fn exec_replace(cmd: &mut Command) -> Result<()> {
        let err = cmd.exec();
        bail!("process did not exit successfully: {err}")
    }
}

#[cfg(windows)]
mod imp {
    use std::process::Command;

    use anyhow::{bail, Context, Result};
    use windows_sys::Win32::Foundation::{BOOL, FALSE, TRUE};
    use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;

    unsafe extern "system" fn ctrlc_handler(_: u32) -> BOOL {
        // Do nothing; let the child process handle it.
        TRUE
    }

    pub fn exec_replace(cmd: &mut Command) -> Result<()> {
        unsafe {
            if SetConsoleCtrlHandler(Some(ctrlc_handler), TRUE) == FALSE {
                panic!("Could not set Ctrl-C handler.");
            }
        }

        // Just execute the process as normal.

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
}

#[tracing::instrument(level = "trace", skip_all)]
pub fn exec(cmd: &mut Command, config: &Config) -> Result<()> {
    exec_piping(
        cmd,
        config,
        |line: &str| {
            debug!("{line}");
        },
        |line: &str| {
            debug!("{line}");
        },
    )
}

/// Runs the process, waiting for completion, and mapping non-success exit codes to an error.
#[tracing::instrument(level = "trace", skip_all)]
pub fn exec_piping(
    cmd: &mut Command,
    config: &Config,
    stdout_callback: impl Fn(&str) + Send,
    stderr_callback: impl Fn(&str) + Send,
) -> Result<()> {
    let cmd_str = shlex_join(cmd);

    config.ui().verbose(Status::new("Running", &cmd_str));
    let _spinner = config.ui().widget(Spinner::new(cmd_str.clone()));

    return thread::scope(move |s| {
        let mut proc = cmd
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| anyhow!("could not execute process: {cmd_str}"))?;

        let span = Arc::new(debug_span!("exec", pid = proc.id()));
        let _enter = span.enter();
        debug!("{cmd_str}");

        let stdout = proc.stdout.take().expect("we asked Rust to pipe stdout");
        s.spawn({
            let span = debug_span!("out");
            move || {
                let mut stdout = stdout;
                pipe(&span, &mut stdout, stdout_callback);
            }
        });

        let stderr = proc.stderr.take().expect("we asked Rust to pipe stderr");
        s.spawn({
            let span = debug_span!("err");
            move || {
                let mut stderr = stderr;
                pipe(&span, &mut stderr, stderr_callback);
            }
        });

        let exit_status = proc
            .wait()
            .with_context(|| anyhow!("could not wait for process termination: {cmd_str}"))?;
        if exit_status.success() {
            Ok(())
        } else {
            bail!("process did not exit successfully: {exit_status}");
        }
    });

    fn pipe(span: &Span, stream: &mut dyn Read, callback: impl Fn(&str)) {
        let _enter = span.enter();
        let stream = BufReader::with_capacity(128, stream);
        for line in stream.lines() {
            match line {
                Ok(line) => callback(line.as_str()),
                Err(err) => warn!("{err:?}"),
            }
        }
    }
}

/// Python's [`shlex.join`] for [`Command`].
///
/// [`shlex.join`]: https://docs.python.org/3/library/shlex.html#shlex.join
fn shlex_join(cmd: &Command) -> String {
    ShlexJoin(cmd).to_string()
}

struct ShlexJoin<'a>(&'a Command);

impl<'a> fmt::Display for ShlexJoin<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn write_quoted(f: &mut fmt::Formatter<'_>, s: &OsStr) -> fmt::Result {
            let utf = s.to_string_lossy();
            if utf.contains('"') {
                write!(f, "{s:?}")
            } else {
                write!(f, "{utf}")
            }
        }

        let cmd = &self.0;
        write_quoted(f, cmd.get_program())?;

        for arg in cmd.get_args() {
            write!(f, " ")?;
            write_quoted(f, arg)?;
        }
        Ok(())
    }
}
