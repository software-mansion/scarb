use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub mod methods;

pub type RequestId = u64;

#[derive(Serialize, Deserialize)]
pub struct RpcResponse {
    pub id: RequestId,
    pub value: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct RpcRequest {
    pub id: RequestId,
    pub method: String,
    pub value: serde_json::Value,
}

pub trait Method {
    const METHOD: &str;

    type Params: Serialize + DeserializeOwned;
    type Response: Serialize + DeserializeOwned;
}
