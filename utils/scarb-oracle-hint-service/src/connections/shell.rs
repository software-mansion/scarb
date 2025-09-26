use crate::connection::Connection;
use crate::protocol::{ConnectCtx, Protocol};
use anyhow::{Context, Result, ensure};
use deno_task_shell::parser::{SequentialList, parse};
use deno_task_shell::{ExecutableCommand, ShellCommand, ShellPipeReader, ShellState, pipe};
use starknet_core::codec::{Decode, Encode};
use starknet_core::types::{ByteArray, Felt};
use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader, Read};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::task::spawn_blocking;
use tracing::{Span, debug, debug_span, warn};

/// The `shell` protocol executes a provided shell invocation line and returns its stdout as
/// a `ByteArray`. Stderr is redirected to the logs.
pub struct Shell;

impl Protocol for Shell {
    const SCHEME: &'static str = "shell";

    #[tracing::instrument(skip_all)]
    fn connect(_command: &str, _ctx: ConnectCtx<'_>) -> Result<Box<dyn Connection + 'static>> {
        Ok(Box::new(ShellConnection::new()?))
    }
}

struct ShellConnection {
    rt: tokio::runtime::Runtime,
}

impl ShellConnection {
    fn new() -> Result<Self> {
        Ok(Self {
            rt: tokio::runtime::Builder::new_current_thread().build()?,
        })
    }
}

impl Connection for ShellConnection {
    fn call(&mut self, selector: &str, calldata: &[Felt]) -> Result<Vec<Felt>> {
        ensure!(selector == "exec", "unsupported selector: {selector}");

        let span = Arc::new(debug_span!("exec", id = next_id()));
        let _enter = span.enter();

        // Decode arguments.
        let mut calldata = calldata.iter();
        let command: String = ByteArray::decode_iter(&mut calldata)?.try_into()?;
        debug!("{command}");

        // Run command.
        let command = parse(&command)?;
        let shell = build_shell()?;
        let (exit_code, stdout) = self.rt.block_on(execute(span.clone(), command, shell));

        // Encode the result.
        let mut felts = vec![Felt::from(exit_code)];
        ByteArray::from(stdout).encode(&mut felts)?;
        Ok(felts)
    }
}

fn build_shell() -> Result<ShellState> {
    let mut custom_commands: HashMap<String, Rc<dyn ShellCommand>> = HashMap::new();

    // If there is a `SCARB` env var present, use it to override the `scarb` command in the shell.
    if let Some(scarb_path) = env::var_os("SCARB") {
        custom_commands.insert(
            "scarb".into(),
            Rc::new(ExecutableCommand::new("scarb".into(), scarb_path.into())),
        );
    }

    Ok(ShellState::new(
        env::vars_os().collect(),
        env::current_dir().context("cannot get current dir")?,
        custom_commands,
        Default::default(),
    ))
}

async fn execute(span: Arc<Span>, command: SequentialList, shell: ShellState) -> (i32, Vec<u8>) {
    // Capture stdout.
    let (stdout_reader, stdout_writer) = pipe();
    let stdout_handle = spawn_blocking(move || {
        let mut buf = Vec::new();
        stdout_reader.pipe_to(&mut buf).unwrap();
        buf
    });

    // Redirect stderr to logs.
    let (ShellPipeReader::OsPipe(mut stderr_reader), stderr_writer) = pipe() else {
        unreachable!();
    };
    let stderr_handle = spawn_blocking(move || {
        let span = debug_span!(parent: &*span, "err");
        pipe_to_tracing(&span, &mut stderr_reader);
    });

    let exit_code = deno_task_shell::execute_with_pipes(
        command,
        shell,
        ShellPipeReader::stdin(),
        stdout_writer,
        stderr_writer,
    )
    .await;

    let stdout = stdout_handle.await.unwrap();
    stderr_handle.await.unwrap();
    (exit_code, stdout)
}

fn pipe_to_tracing(span: &Span, stream: &mut dyn Read) {
    let _enter = span.enter();
    let stream = BufReader::with_capacity(128, stream);
    for line in stream.lines() {
        match line {
            Ok(line) => debug!("{line}"),
            Err(err) => warn!("{err:?}"),
        }
    }
}

fn next_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}
