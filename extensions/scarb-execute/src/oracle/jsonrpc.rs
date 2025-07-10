//! Simple JSON-RPC types that suit our needs and nothing more.
//! It is surprising how complex existing JSON-RPC Rust implementations are,
//! we don't need any kinds of async, extendibility or network stuff that they provide.

use anyhow::{Result, bail};
use serde::de::{Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

impl Message {
    pub fn kind(&self) -> &'static str {
        match self {
            Message::Request(_) => "request",
            Message::Response(_) => "response",
            Message::Notification(_) => "notification",
        }
    }

    pub fn expect_request(self) -> Result<Request> {
        match self {
            Message::Request(request) => Ok(request),
            _ => bail!("expected request, got {}", self.kind()),
        }
    }

    pub fn expect_response(self) -> Result<Response> {
        match self {
            Message::Response(response) => Ok(response),
            _ => bail!("expected response, got {}", self.kind()),
        }
    }

    pub fn expect_notification(self) -> Result<Notification> {
        match self {
            Message::Notification(notification) => Ok(notification),
            _ => bail!("expected notification, got {}", self.kind()),
        }
    }
}

impl From<Request> for Message {
    fn from(request: Request) -> Message {
        Message::Request(request)
    }
}

impl From<Response> for Message {
    fn from(response: Response) -> Message {
        Message::Response(response)
    }
}

impl From<Notification> for Message {
    fn from(notification: Notification) -> Message {
        Message::Notification(notification)
    }
}

impl TryFrom<Message> for Request {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        message.expect_request()
    }
}

impl TryFrom<Message> for Response {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        message.expect_response()
    }
}

impl TryFrom<Message> for Notification {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        message.expect_notification()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Request {
    pub jsonrpc: TwoPointZero,
    pub id: Id,
    pub method: String,
    #[serde(default = "serde_json::Value::default")]
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Response {
    pub jsonrpc: TwoPointZero,
    pub id: Id,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Notification {
    pub jsonrpc: TwoPointZero,
    pub method: String,
    #[serde(default = "serde_json::Value::default")]
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub params: serde_json::Value,
}

/// JSON-RPC v2 marker type.
///
/// Copied verbatim from `jsonrpsee-types` crate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TwoPointZero;

struct TwoPointZeroVisitor;

impl Visitor<'_> for TwoPointZeroVisitor {
    type Value = TwoPointZero;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(r#"a string "2.0""#)
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match s {
            "2.0" => Ok(TwoPointZero),
            _ => Err(de::Error::invalid_value(Unexpected::Str(s), &self)),
        }
    }
}

impl<'de> Deserialize<'de> for TwoPointZero {
    fn deserialize<D>(deserializer: D) -> Result<TwoPointZero, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(TwoPointZeroVisitor)
    }
}

impl Serialize for TwoPointZero {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("2.0")
    }
}

/// Request ID.
#[derive(PartialEq, Clone, Hash, Eq, Deserialize, Serialize, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum Id {
    Null,
    Number(u64),
    String(String),
}

impl From<()> for Id {
    fn from(_: ()) -> Self {
        Self::Null
    }
}

impl From<u64> for Id {
    fn from(id: u64) -> Self {
        Self::Number(id)
    }
}

impl From<String> for Id {
    fn from(id: String) -> Self {
        Self::String(id)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::Null => f.write_str("null"),
            Self::Number(id) => fmt::Display::fmt(id, f),
            Self::String(id) => fmt::Debug::fmt(id, f),
        }
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({self})")
    }
}

#[derive(Default)]
pub struct MessageFactory {
    next_id: u64,
}

impl MessageFactory {
    pub fn request(&mut self, method: impl Into<String>, params: impl Serialize) -> Request {
        Request {
            jsonrpc: TwoPointZero,
            id: self.next_id(),
            method: method.into(),
            params: serde_json::to_value(params).unwrap(),
        }
    }

    pub fn notification(
        &mut self,
        method: impl Into<String>,
        params: impl Serialize,
    ) -> Notification {
        Notification {
            jsonrpc: TwoPointZero,
            method: method.into(),
            params: serde_json::to_value(params).unwrap(),
        }
    }

    pub fn result(&self, request: &Request, result: impl Serialize) -> Response {
        Response {
            jsonrpc: TwoPointZero,
            id: request.id.clone(),
            result: Some(serde_json::to_value(result).unwrap()),
            error: None,
        }
    }

    pub fn error(&self, request: &Request, error: ResponseError) -> Response {
        Response {
            jsonrpc: TwoPointZero,
            id: request.id.clone(),
            result: None,
            error: Some(error),
        }
    }

    fn next_id(&mut self) -> Id {
        let id = Id::Number(self.next_id);
        self.next_id += 1;
        id
    }
}
