use std::ops::{Deref, DerefMut};
use std::process;
use std::process::Stdio;
use std::time::Duration;

use assert_fs::TempDir;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::time::timeout;
use url::Url;

use scarb_test_support::command::Scarb;

const TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn run() {
    let t = TempDir::new().unwrap();

    let mut proc = KillOnDrop(
        Command::from(Scarb::new().std())
            .arg("cairo-language-server")
            .current_dir(&t)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("failed to start ls"),
    );
    let mut stdin = proc.stdin.take().unwrap();
    let mut stdout = BufReader::new(proc.stdout.take().unwrap());

    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "capabilities": {},
                "initializationOptions": {},
                "processId": process::id(),
                "rootUri": Url::from_directory_path(t.path()).unwrap(),
                "trace": "off",
            }
        }),
    )
    .await;

    {
        let mut res = read(&mut stdout).await;
        assert!(res.as_object_mut().unwrap().remove("result").is_some());
        assert_eq!(res, json!({"jsonrpc": "2.0", "id": 1}));
    }

    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "initialized",
        }),
    )
    .await;

    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "shutdown",
        }),
    )
    .await;

    assert_eq!(
        read(&mut stdout).await,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "client/registerCapability",
            "params": {"registrations": []}
        })
    );

    assert_eq!(
        read(&mut stdout).await,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": null,
        })
    );

    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
        }),
    )
    .await;

    drop(stdin);
    drop(stdout);
    let exit = timeout(TIMEOUT, proc.wait())
        .await
        .expect("waiting for ls to exit timed out")
        .expect("failed to wait for ls");
    assert!(exit.success(), "ls process crashed: {exit:?}");
}

async fn send(stdin: &mut ChildStdin, input: serde_json::Value) {
    let mut serialized = serde_json::to_string_pretty(&input).unwrap();
    serialized.push_str("\n\n");

    let content_length = serialized.len();

    timeout(
        TIMEOUT,
        stdin.write_all(format!("Content-Length: {content_length}\r\n\r\n{serialized}").as_bytes()),
    )
    .await
    .expect("writing request timed out")
    .expect("failed to write request")
}

async fn read(stdout: &mut BufReader<ChildStdout>) -> serde_json::Value {
    let mut content_length: Option<usize> = None;
    let mut line = String::new();
    loop {
        line.clear();
        timeout(TIMEOUT, stdout.read_line(&mut line))
            .await
            .expect("reading response header timed out")
            .expect("failed to read response header");
        let line = line.trim();
        if line.is_empty() {
            break;
        }
        if let Some(("Content-Length", value)) = line.split_once(": ") {
            content_length = Some(
                value
                    .parse()
                    .expect("invalid Content-Length header in response"),
            );
        }
    }
    let content_length = content_length.expect("missing Content-Length header in response");

    let mut buf = vec![0; content_length];
    timeout(TIMEOUT, stdout.read_exact(&mut buf))
        .await
        .expect("reading response timed out")
        .expect("failed to read response");
    serde_json::from_slice(&buf).expect("failed to parse response")
}

struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.start_kill();
    }
}

impl Deref for KillOnDrop {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for KillOnDrop {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
