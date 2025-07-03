use crate::oracle::connection::Connection;
use anyhow::{Context, Result, anyhow, bail};
use cairo_vm::Felt252;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdout, Command, Stdio};
use url::Url;

/// A struct representing the JSON-RPC version that always serializes to "2.0"
/// and expects "2.0" during deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct JsonRpcVersion;

impl Serialize for JsonRpcVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("2.0")
    }
}

impl<'de> Deserialize<'de> for JsonRpcVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == "2.0" {
            Ok(JsonRpcVersion)
        } else {
            Err(serde::de::Error::custom(format!(
                "expected JSON-RPC version '2.0', got '{}'",
                s
            )))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: JsonRpcVersion,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: JsonRpcVersion,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct InvokeParams {
    selector: String,
    calldata: Vec<i64>,
}

pub struct StdioJsonRpcConnection {
    process: Child,
    stdout_reader: BufReader<ChildStdout>,
    next_id: u64,
}

impl StdioJsonRpcConnection {
    pub fn connect(connection_url: Url) -> Result<Self> {
        // Extract the path from the URL
        let path = connection_url.path();
        if path.is_empty() {
            bail!("stdio connection url must contain a path to executable");
        }

        // Spawn the oracle process
        let mut process = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to spawn oracle process: {path}"))?;

        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| anyhow!("failed to get stdout from oracle process"))?;
        let stdout_reader = BufReader::new(stdout);

        let mut connection = Self {
            process,
            stdout_reader,
            next_id: 0,
        };

        // Wait for the ready message from the oracle
        connection.wait_for_ready()?;

        Ok(connection)
    }

    fn wait_for_ready(&mut self) -> Result<()> {
        // Read the first byte to detect protocol
        let mut first_byte = [0u8; 1];
        use std::io::Read;
        self.stdout_reader
            .read_exact(&mut first_byte)
            .context("failed to read first byte from oracle")?;

        // Check if it starts with '{' for JSON-RPC
        if first_byte[0] != b'{' {
            // Kill the misbehaving process
            let _ = self.process.kill();
            let _ = self.process.wait();
            bail!(
                "oracle process is misbehaving: expected JSON-RPC message starting with '{{', got byte: {}",
                first_byte[0]
            );
        }

        // Read the rest of the line
        let mut line = String::new();
        self.stdout_reader
            .read_line(&mut line)
            .context("failed to read ready message from oracle")?;

        // Prepend the '{' we already read
        let full_line = format!("{{{}", line);

        let ready_msg: JsonRpcRequest = serde_json::from_str(&full_line.trim())
            .context("failed to parse ready message from oracle")?;

        if ready_msg.method != "ready" {
            bail!(
                "expected ready message from oracle, got: {}",
                ready_msg.method
            );
        }

        // Send ready response
        let response = JsonRpcResponse {
            jsonrpc: JsonRpcVersion,
            id: ready_msg.id,
            result: Some(serde_json::json!({})),
            error: None,
        };

        self.send_message(&response)?;
        Ok(())
    }

    fn send_message<T: Serialize>(&mut self, message: &T) -> Result<()> {
        let stdin = self
            .process
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("failed to get stdin from oracle process"))?;

        let json = serde_json::to_string(message).context("failed to serialize message")?;

        writeln!(stdin, "{}", json).context("failed to write message to oracle process")?;

        stdin.flush().context("failed to flush stdin")?;

        Ok(())
    }

    fn read_response(&mut self) -> Result<JsonRpcResponse> {
        let mut line = String::new();

        self.stdout_reader
            .read_line(&mut line)
            .context("failed to read response from oracle")?;

        serde_json::from_str(&line.trim()).context("failed to parse response from oracle")
    }

    fn get_next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }
}

impl Connection for StdioJsonRpcConnection {
    fn call(&mut self, selector: &str, calldata: &[Felt252]) -> Result<Vec<Felt252>> {
        // Convert Felt252 calldata to i64 for JSON serialization
        let calldata_i64: Vec<i64> = calldata
            .iter()
            .map(|felt| {
                // Convert Felt252 to i64, handling potential overflow
                let bytes = felt.to_bytes_be();
                // Take the last 8 bytes and convert to i64
                let mut arr = [0u8; 8];
                let start = if bytes.len() >= 8 { bytes.len() - 8 } else { 0 };
                let copy_len = std::cmp::min(8, bytes.len());
                arr[8 - copy_len..].copy_from_slice(&bytes[start..start + copy_len]);
                i64::from_be_bytes(arr)
            })
            .collect();

        let params = InvokeParams {
            selector: selector.to_string(),
            calldata: calldata_i64,
        };

        let request = JsonRpcRequest {
            jsonrpc: JsonRpcVersion,
            id: self.get_next_id(),
            method: "invoke".to_string(),
            params: Some(serde_json::to_value(params)?),
        };

        // Send the request
        self.send_message(&request)?;

        // Read the response
        let response = self.read_response()?;

        if response.id != request.id {
            bail!(
                "response ID mismatch: expected {}, got {}",
                request.id,
                response.id
            );
        }

        if let Some(error) = response.error {
            bail!("oracle error: {}", error.message);
        }

        let result = response
            .result
            .ok_or_else(|| anyhow!("oracle response missing result"))?;

        // Parse the result as an array of numbers and convert to Felt252
        let result_array: Vec<i64> = serde_json::from_value(result)
            .context("failed to parse oracle result as array of numbers")?;

        let felt_result: Vec<Felt252> = result_array
            .iter()
            .map(|&num| Felt252::from(num as u64))
            .collect();

        Ok(felt_result)
    }
}

impl Drop for StdioJsonRpcConnection {
    fn drop(&mut self) {
        // Send shutdown message
        let shutdown_request = JsonRpcRequest {
            jsonrpc: JsonRpcVersion,
            id: self.get_next_id(),
            method: "shutdown".to_string(),
            params: None,
        };

        let _ = self.send_message(&shutdown_request);

        // Wait for process to terminate
        let _ = self.process.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_version_serialization() {
        let version = JsonRpcVersion;
        let serialized = serde_json::to_string(&version).unwrap();
        assert_eq!(serialized, "\"2.0\"");
    }

    #[test]
    fn test_jsonrpc_version_deserialization() {
        let json = "\"2.0\"";
        let version: JsonRpcVersion = serde_json::from_str(json).unwrap();
        assert_eq!(version, JsonRpcVersion);
    }

    #[test]
    fn test_jsonrpc_version_deserialization_invalid() {
        let json = "\"1.0\"";
        let result: Result<JsonRpcVersion, _> = serde_json::from_str(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected JSON-RPC version '2.0', got '1.0'"));
    }

    #[test]
    fn test_jsonrpc_request_serialization() {
        let request = JsonRpcRequest {
            jsonrpc: JsonRpcVersion,
            id: 1,
            method: "test".to_string(),
            params: None,
        };
        let serialized = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
        assert_eq!(parsed["method"], "test");
    }

    #[test]
    fn test_jsonrpc_response_serialization() {
        let response = JsonRpcResponse {
            jsonrpc: JsonRpcVersion,
            id: 1,
            result: Some(serde_json::json!({"success": true})),
            error: None,
        };
        let serialized = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
        assert_eq!(parsed["result"]["success"], true);
    }
}
