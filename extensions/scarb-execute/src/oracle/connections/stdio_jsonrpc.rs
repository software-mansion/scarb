use crate::oracle::connection::Connection;
use crate::oracle::jsonrpc;
use anyhow::{Context, Result, anyhow, bail, ensure};
use cairo_vm::Felt252;
use camino::Utf8Path;
use serde::Serialize;
use serde_json::Value::Null;
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdout, Command, Stdio};
use tracing::{trace, warn};
use url::Url;

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
    #[tracing::instrument(skip_all, fields(url = %connection_url))]
    pub fn connect(connection_url: Url) -> Result<Self> {
        let path = parse_connection_url(&connection_url)?;

        let io = Io::spawn(
            Command::new(path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                // TODO(next PR): Pipe stderr to logs like in scarb::process::exec_piping.
                .stderr(Stdio::inherit()),
        )?;

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
        let mut process = command.spawn().context("failed to spawn oracle process")?;

        let stdout = BufReader::new(
            process
                .stdout
                .take()
                .expect("failed to get stdout from oracle process"),
        );

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

    fn send(&mut self, message: impl Into<jsonrpc::Message>) -> Result<()> {
        let message = message.into();
        return inner(self, &message);

        // Minimize monomorphisation effects.
        fn inner(this: &mut Io, message: &jsonrpc::Message) -> Result<()> {
            let stdin = this
                .process
                .stdin
                .as_mut()
                .ok_or_else(|| anyhow!("oracle is already dead"))?;

            // Serialise the message to string as a whole to avoid emitting unterminated messages.
            let line =
                serde_json::to_string(&message).context("failed to serialize message to oracle")?;

            trace!("send: {line}");
            writeln!(stdin, "{line}").context("failed to write message to oracle")?;
            stdin.flush().context("failed to flush oracle stdin")?;

            Ok(())
        }
    }

    fn recv(&mut self) -> Option<Result<jsonrpc::Message>> {
        let mut buf = String::new();
        match self.stdout.read_line(&mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                trace!("recv: {buf}");
                Some(
                    serde_json::from_str(buf.trim()).context("failed to parse message from oracle"),
                )
            }
            Err(err) => Some(Err(err).context("failed to read bytes from oracle")),
        }
    }
}

fn parse_connection_url(url: &Url) -> Result<&Utf8Path> {
    // This is guaranteed by scheme routing logic.
    assert_eq!(url.scheme(), "stdio");

    ensure!(!url.has_authority(), "authority not allowed in oracle url");
    ensure!(
        url.query().is_none(),
        "query parameters not allowed in oracle url"
    );
    ensure!(
        url.fragment().is_none(),
        "fragments not allowed in oracle url"
    );

    Ok(Utf8Path::new(url.path()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_connection_url() {
        fn check(url: &str, expected: Result<&str, &str>) {
            let url = Url::parse(url).unwrap();
            let actual = parse_connection_url(&url).map_err(|e| e.to_string());
            let actual = actual.as_ref().map(|p| p.as_str()).map_err(|e| e.as_str());
            assert_eq!(actual, expected);
        }

        check("stdio:/foo/bar.txt", Ok("/foo/bar.txt"));
        check("stdio:foo/bar.txt", Ok("foo/bar.txt"));
        check("stdio:./foo/bar.txt", Ok("./foo/bar.txt"));
        check("stdio:/", Ok("/"));
        check("stdio:foo", Ok("foo"));

        check(
            "stdio://host/user@host/path",
            Err("authority not allowed in oracle url"),
        );
        check(
            "stdio://user:pass@host/path",
            Err("authority not allowed in oracle url"),
        );
        check(
            "stdio:/path?query=1",
            Err("query parameters not allowed in oracle url"),
        );
        check(
            "stdio:host/path?name=value",
            Err("query parameters not allowed in oracle url"),
        );
        check(
            "stdio:/path#fragment",
            Err("fragments not allowed in oracle url"),
        );
        check(
            "stdio:host/path#section",
            Err("fragments not allowed in oracle url"),
        );
    }
}
