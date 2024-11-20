use serde::{Deserialize, Serialize};

/// Represents a unique identifier for an RPC request.
pub type RequestId = u64;

/// Standard format for responses sent from an RPC server.
///
/// Represents the completion of a request, which could yield a result or an error.
#[derive(Serialize, Deserialize)]
pub struct RpcResponse {
    /// The identifier of the request to which this response corresponds.
    pub id: RequestId,
    /// The result of the RPC request if successful.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Details about the error if the request failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

impl RpcResponse {
    /// Converts this response into a [`Result`], containing either the successful result
    /// or the error that occurred during the RPC request.
    ///
    /// # Returns
    /// * `Ok(serde_json::Value)` if the RPC request was successful and contains a result.
    /// * `Err(ResponseError)` if the RPC request failed and contains an error.
    pub fn into_result(self) -> Result<serde_json::Value, ResponseError> {
        match (self.result, self.error) {
            (_, Some(error)) => Err(error),
            (Some(response), None) => Ok(response),
            (None, None) => Err(ResponseError {
                message: "`RpcResponse` is missing both `result` and `error`.".to_string(),
            }),
        }
    }
}

/// Describes errors that occurred during an RPC operation.
///
/// Provides an error message detailing what went wrong during the request.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseError {
    /// A human-readable message providing more details about the error.
    pub message: String,
}

/// Structure defining an RPC request.
///
/// Contains necessary information for the server to process the request and generate a response.
#[derive(Debug, Serialize, Deserialize)]
pub struct RpcRequest {
    /// The identifier of the request, used to match the response with the request.
    pub id: RequestId,
    /// The name of the method to be invoked.
    pub method: String,
    /// The parameters for the method call, packaged into a JSON value.
    pub value: serde_json::Value,
}
