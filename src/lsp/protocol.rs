use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(u64);

impl RequestId {
    pub fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Serialize)]
pub struct RequestMessage<P> {
    jsonrpc: &'static str,
    id: RequestId,
    method: &'static str,
    params: P,
}

impl<P> RequestMessage<P> {
    pub fn new(id: RequestId, method: &'static str, params: P) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method,
            params,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NotificationMessage<P> {
    jsonrpc: &'static str,
    method: &'static str,
    params: P,
}

impl<P> NotificationMessage<P> {
    pub fn new(method: &'static str, params: P) -> Self {
        Self {
            jsonrpc: "2.0",
            method,
            params,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ResponseMessage<R> {
    jsonrpc: &'static str,
    id: Value,
    result: R,
}

impl<R> ResponseMessage<R> {
    pub fn success(id: Value, result: R) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result,
        }
    }
}

#[derive(Debug)]
pub enum IncomingMessage {
    Request(ServerRequest),
    Notification(ServerNotification),
    Response(ServerResponse),
}

#[derive(Debug)]
pub struct ServerRequest {
    pub id: Value,
}

#[derive(Debug)]
pub struct ServerNotification {
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug)]
pub struct ServerResponse {
    pub id: RequestId,
    pub result: Option<Value>,
    pub error: Option<ResponseError>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResponseError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

pub fn parse_incoming_message(value: Value) -> Result<IncomingMessage> {
    if let Some(method) = value.get("method").and_then(Value::as_str) {
        let params = value.get("params").cloned();

        if let Some(id) = value.get("id") {
            let _ = params;
            return Ok(IncomingMessage::Request(ServerRequest { id: id.clone() }));
        }

        return Ok(IncomingMessage::Notification(ServerNotification {
            method: method.to_string(),
            params,
        }));
    }

    if let Some(id) = value.get("id") {
        let request_id = serde_json::from_value::<RequestId>(id.clone())?;
        let error = value
            .get("error")
            .cloned()
            .map(serde_json::from_value::<ResponseError>)
            .transpose()?;

        return Ok(IncomingMessage::Response(ServerResponse {
            id: request_id,
            result: value.get("result").cloned(),
            error,
        }));
    }

    bail!("unrecognized JSON-RPC message: {value}")
}
