use serde::{Deserialize, Serialize};

pub type RequestId = u64;

#[derive(Serialize, Deserialize)]
pub struct RpcResponse {
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseError {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcRequest {
    pub id: RequestId,
    pub method: String,
    pub value: serde_json::Value,
}
