use anyhow::{Context, Result, anyhow, bail};
use std::error::Error;
use std::ffi::OsStr;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::{fmt, iter, mem, thread};
use tracing::{Span, debug, debug_span, warn};

use scarb_ui::components::{Spinner, Status};

use crate::core::Config;
pub use crate::internal::fsx::is_executable;

/// Replaces the current process with the target process.
///
/// # Flow
/// This function just throws a special [`WillExecReplace`] error used to perform the syscall at
/// the very end of the `main` function. This allows running all destructors waiting higher up
/// the call stack.
///
/// # Implementation details
/// On Unix, this executes the process using the Unix syscall `execvp`, which will block this
/// process and will only return if there is an error.
///
/// On Windows this isn't technically possible. Instead, we emulate it to the best of our ability.
/// One aspect we fix here is that we specify a handler for the Ctrl-C handler.
/// In doing so (and by effectively ignoring it) we should emulate proxying Ctrl-C handling to
/// the application at hand, which will either terminate or handle it itself.
/// According to Microsoft's documentation at
/// <https://docs.microsoft.com/en-us/windows/console/ctrl-c-and-ctrl-break-signals>
/// the Ctrl-C signal is sent to all processes attached to a terminal, which should include our
/// child process. If the child terminates, then we will terminate Scarb quickly and silently,
/// and if the child handles the signal, then we won't terminate (and we shouldn't!) until
/// the process itself later exits.
///
/// # Drop semantics
/// [`WillExecReplace`] implements the _Drop Bomb_ pattern, which means it will panic if dropped
/// without [`WillExecReplace::take_over`] being called.
pub fn exec_replace(command: Command) -> Result<()> {
    let err = WillExecReplace(Some(Box::new(command)));
    debug!("{err}");
    Err(err.into())
}

/// Error object thrown from [`exec_replace`].
///
/// See [`exec_replace`] for details.
#[derive(Debug)]
pub struct WillExecReplace(Option<Box<Command>>);

impl WillExecReplace {
    /// Performs or emulates process replacement and defuses the drop bomb.
    pub fn take_over(mut self) -> ! {
        let command = self.0.take().unwrap();
        mem::forget(self); // Defuse the drop bomb.
        imp::exec_replace(*command);
    }
}

impl fmt::Display for WillExecReplace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EXEC")?;
        f.write_str(&shlex_join(self.0.as_ref().unwrap()))?;
        Ok(())
    }
}

impl Error for WillExecReplace {}

impl Drop for WillExecReplace {
    fn drop(&mut self) {
        panic!("not defused: {self}");
    }
}

#[cfg(unix)]
mod imp {
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    pub fn exec_replace(mut cmd: Command) -> ! {
        let err = cmd.exec();
        panic!("{err}");
    }
}

#[cfg(windows)]
mod imp {
    use std::process;
    use std::process::Command;

    use windows_sys::Win32::Foundation::{BOOL, FALSE, TRUE};
    use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;

    unsafe extern "system" fn ctrlc_handler(_: u32) -> BOOL {
        // Do nothing; let the child process handle it.
        TRUE
    }

    pub fn exec_replace(mut cmd: Command) -> ! {
        unsafe {
            if SetConsoleCtrlHandler(Some(ctrlc_handler), TRUE) == FALSE {
                panic!("Could not set Ctrl-C handler.");
            }
        }

        // Execute the process as normal.

        let exit_status = cmd
            .spawn()
            .expect("failed to spawn a subprocess")
            .wait()
            .expect("failed to wait for the subprocess process to finish");

        let exit_code = exit_status.code().unwrap_or(1);
        process::exit(exit_code);
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

fn shlex_join(cmd: &Command) -> String {
    shell_words::join(
        iter::once(cmd.get_program())
            .chain(cmd.get_args())
            .map(OsStr::to_string_lossy),
    )
}
