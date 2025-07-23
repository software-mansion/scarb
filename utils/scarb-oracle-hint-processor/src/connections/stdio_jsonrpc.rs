use crate::connection::Connection;
use crate::jsonrpc;
use anyhow::{Context, Result, anyhow, bail, ensure};
use cairo_vm::Felt252;
use serde::Serialize;
use serde_json::Value::Null;
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::thread;
use tracing::{debug, debug_span, trace, warn};

#[derive(Serialize)]
struct InvokeParams {
    selector: String,
    calldata: Vec<Felt252>,
}

pub struct StdioJsonRpcConnection {
    io: Io,
    mf: jsonrpc::MessageFactory,
}

impl StdioJsonRpcConnection {
    #[tracing::instrument]
    pub fn connect(command: &str) -> Result<Self> {
        let command_words =
            shell_words::split(command).context("failed to parse oracle command")?;
        let (command, args) = command_words
            .split_first()
            .ok_or_else(|| anyhow!("empty oracle command"))?;

        let io = Io::spawn(Command::new(command).args(args))?;

        let mut connection = Self {
            io,
            mf: Default::default(),
        };

        connection.initialize()?;

        Ok(connection)
    }

    fn initialize(&mut self) -> Result<()> {
        // Read the first byte to ensure this is JSON-RPC protocol.
        // This is future-proofing to enable introducing other over-stdio transports
        // with auto-detection capabilities.
        let first_byte = self.io.peek()?;
        if first_byte != Some(b'{') {
            self.io.terminate()?;

            match first_byte {
                Some(byte) => bail!(
                    "oracle process is misbehaving: expected JSON-RPC message starting with '{{', got byte: '{}'",
                    std::ascii::escape_default(byte)
                ),
                None => bail!("oracle process is misbehaving: no bytes received"),
            }
        }

        // Handle `ready` flow.
        let ready_request = self
            .io
            .recv()
            .ok_or_else(|| anyhow!("expected ready request from oracle"))??
            .expect_request()
            .context("expected ready request from oracle")?;

        if ready_request.method != "ready" {
            const MESSAGE: &str = "expected ready request from oracle";
            self.io.send(self.mf.error(
                &ready_request,
                jsonrpc::ResponseError {
                    code: 0,
                    message: MESSAGE.into(),
                    data: None,
                },
            ))?;
            bail!("{MESSAGE}");
        }

        // We ignore ready params here, but check if they follow the spec nonetheless.
        if !ready_request.params.is_null() && !ready_request.params.is_object() {
            const MESSAGE: &str = "invalid ready request params, expected null or object";
            self.io.send(self.mf.error(
                &ready_request,
                jsonrpc::ResponseError {
                    code: 0,
                    message: MESSAGE.into(),
                    data: None,
                },
            ))?;
            bail!("{MESSAGE}");
        }

        self.io.send(self.mf.result(&ready_request, json!({})))?;

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn shutdown(&mut self) -> Result<()> {
        self.io.send(self.mf.notification("shutdown", Null))?;
        self.io.terminate()?;
        Ok(())
    }
}

impl Connection for StdioJsonRpcConnection {
    #[tracing::instrument(skip(self, calldata))]
    fn call(&mut self, selector: &str, calldata: &[Felt252]) -> Result<Vec<Felt252>> {
        let invoke_request = self.mf.request(
            "invoke",
            InvokeParams {
                selector: selector.into(),
                calldata: calldata.into(),
            },
        );
        let id = invoke_request.id.clone();
        self.io.send(invoke_request)?;

        let invoke_response = self
            .io
            .recv()
            .ok_or_else(|| anyhow!("oracle process terminated"))??
            .expect_response()?;
        ensure!(
            invoke_response.id == id,
            "response ID mismatch: expected {}, got {}",
            id,
            invoke_response.id
        );

        if let Some(error) = invoke_response.error {
            bail!("{}", error.message);
        }

        if let Some(result) = invoke_response.result {
            Ok(serde_json::from_value(result).context("failed to parse oracle result")?)
        } else {
            Ok(vec![])
        }
    }
}

impl Drop for StdioJsonRpcConnection {
    fn drop(&mut self) {
        if let Err(err) = self.shutdown().context("failed to shutdown oracle") {
            warn!("{err:?}")
        }
    }
}

struct Io {
    process: Child,
    stdout: BufReader<ChildStdout>,
}

impl Io {
    fn spawn(command: &mut Command) -> Result<Self> {
        let mut process = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to spawn oracle process")?;

        let stdout = BufReader::new(
            process
                .stdout
                .take()
                .expect("failed to get stdout from oracle process"),
        );

        let stderr = BufReader::new(
            process
                .stderr
                .take()
                .expect("failed to get stderr from oracle process"),
        );

        let err_span = debug_span!("err");
        thread::spawn(move || {
            let _span = err_span.enter();
            for line in stderr.lines() {
                match line {
                    Ok(line) => debug!("{line}"),
                    Err(err) => warn!("{err:?}"),
                }
            }
        });

        Ok(Self { process, stdout })
    }

    fn peek(&mut self) -> Result<Option<u8>> {
        let buf = self
            .stdout
            .fill_buf()
            .context("failed to read oracle stdout")?;
        Ok(buf.first().copied())
    }

    fn terminate(&mut self) -> Result<()> {
        self.process.kill()?;
        self.process.wait()?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn send(&mut self, message: impl Into<jsonrpc::Message>) -> Result<()> {
        let stdin = self
            .process
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("oracle is already dead"))?;

        // Serialise the message to string as a whole to avoid emitting unterminated messages.
        let line = serde_json::to_string(&message.into())
            .context("failed to serialize message to oracle")?;

        trace!("{line}");
        writeln!(stdin, "{line}").context("failed to write message to oracle")?;
        stdin.flush().context("failed to flush oracle stdin")?;

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn recv(&mut self) -> Option<Result<jsonrpc::Message>> {
        let mut buf = String::new();
        match self.stdout.read_line(&mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                trace!("{}", buf.trim_end_matches('\n'));
                Some(
                    serde_json::from_str(buf.trim()).context("failed to parse message from oracle"),
                )
            }
            Err(err) => Some(Err(err).context("failed to read bytes from oracle")),
        }
    }
}
