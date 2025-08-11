use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Smoke tests that `test_oracle.py` actually works as intended.
#[test]
fn oracle_json_rpc_smoke_test() {
    // Spawn test_oracle.py process and grab it's I/O.
    let mut process = Command::new("python3")
        .arg("src/test_oracle.py")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();

    let mut stdin = process.stdin.take().unwrap();
    let stdout = process.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    let mut send = |msg: &str| {
        writeln!(stdin, "{msg}").unwrap();
    };

    let mut recv = |expected_msg: &str| {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        line = line.trim_end().to_string();
        assert_eq!(line, expected_msg);
    };

    // Communication sequence that we expect to function properly.
    recv(r#"{"jsonrpc": "2.0", "id": 0, "method": "ready"}"#);
    send(r#"{"jsonrpc": "2.0", "id": 0, "result": {}}"#);

    send(
        r#"{"jsonrpc": "2.0", "id": 0, "method": "invoke", "params": {"selector": "sqrt", "calldata": ["0x10"]}}"#,
    );
    recv(r#"{"jsonrpc": "2.0", "id": 0, "result": ["0x4"]}"#);

    send(
        r#"{"jsonrpc": "2.0", "id": 1, "method": "invoke", "params": {"selector": "panic", "calldata": []}}"#,
    );
    recv(r#"{"jsonrpc": "2.0", "id": 1, "error": {"code": 0, "message": "oops"}}"#);

    send(r#"{"jsonrpc": "2.0", "method": "shutdown"}"#);

    // Close stdin to the signal end of the input.
    drop(stdin);

    // Wait for a process to terminate.
    let status = process.wait().unwrap();
    assert!(status.success(), "oracle process should exit successfully");
}
