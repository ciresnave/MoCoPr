//! Protocol handling for MCP
//!
//! This module provides high-level protocol handling for MCP communications,
//! including message routing, capability negotiation, and error handling.

use crate::{Error, Result, types::*};
use serde_json::Value;
use uuid::Uuid;

pub mod handler;
pub mod router;
pub mod session;

#[cfg(test)]
mod tests;

pub use handler::*;
pub use router::*;
pub use session::*;

/// Protocol version constants
pub const PROTOCOL_VERSION: &str = "2025-06-18";
/// List of protocol versions supported by this implementation
pub const SUPPORTED_VERSIONS: &[&str] = &["2025-06-18"];

/// JSON-RPC error codes
pub mod error_codes {
    /// Invalid JSON was received by the server
    pub const PARSE_ERROR: i32 = -32700;
    /// The JSON sent is not a valid Request object
    pub const INVALID_REQUEST: i32 = -32600;
    /// The method does not exist / is not available
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid method parameter(s)
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal JSON-RPC error
    pub const INTERNAL_ERROR: i32 = -32603;

    // MCP-specific error codes
    /// The requested capability is not supported by the server
    pub const CAPABILITY_NOT_SUPPORTED: i32 = -32000;
    /// The requested resource was not found
    pub const RESOURCE_NOT_FOUND: i32 = -32001;
    /// The requested tool was not found
    pub const TOOL_NOT_FOUND: i32 = -32002;
    /// The requested prompt was not found
    pub const PROMPT_NOT_FOUND: i32 = -32003;
    /// The client does not have permission to perform the requested operation
    pub const PERMISSION_DENIED: i32 = -32004;
    /// The client has been rate limited
    pub const RATE_LIMITED: i32 = -32005;
}

/// Protocol utilities
pub struct Protocol;

impl Protocol {
    /// Check if a protocol version is supported
    pub fn is_version_supported(version: &str) -> bool {
        SUPPORTED_VERSIONS.contains(&version)
    }

    /// Get the latest supported protocol version
    pub fn latest_version() -> &'static str {
        PROTOCOL_VERSION
    }

    /// Create a JSON-RPC request
    pub fn create_request(
        method: &str,
        params: Option<Value>,
        id: Option<RequestId>,
    ) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        }
    }

    /// Create a JSON-RPC response
    pub fn create_response(
        id: Option<RequestId>,
        result: Option<Value>,
        error: Option<JsonRpcError>,
    ) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result,
            error,
        }
    }

    /// Create a JSON-RPC notification
    pub fn create_notification(method: &str, params: Option<Value>) -> JsonRpcNotification {
        JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        }
    }

    /// Create a JSON-RPC error
    pub fn create_error(code: i32, message: &str, data: Option<Value>) -> JsonRpcError {
        JsonRpcError {
            code,
            message: message.to_string(),
            data,
        }
    }

    /// Generate a unique request ID
    pub fn generate_request_id() -> RequestId {
        RequestId::from(Uuid::new_v4())
    }

    /// Parse a JSON-RPC message from string
    pub fn parse_message(message: &str) -> Result<JsonRpcMessage> {
        let value: Value = serde_json::from_str(message)?;

        // Check if it's a request, response, or notification
        if value.get("method").is_some() {
            if value.get("id").is_some() {
                // It's a request
                let request: JsonRpcRequest = serde_json::from_value(value)?;
                Ok(JsonRpcMessage::Request(request))
            } else {
                // It's a notification
                let notification: JsonRpcNotification = serde_json::from_value(value)?;
                Ok(JsonRpcMessage::Notification(notification))
            }
        } else if value.get("result").is_some() || value.get("error").is_some() {
            // It's a response
            let response: JsonRpcResponse = serde_json::from_value(value)?;
            Ok(JsonRpcMessage::Response(response))
        } else {
            Err(Error::InvalidRequest(
                "Invalid JSON-RPC message format".to_string(),
            ))
        }
    }

    /// Serialize a JSON-RPC message to string
    pub fn serialize_message(message: &JsonRpcMessage) -> Result<String> {
        match message {
            JsonRpcMessage::Request(req) => serde_json::to_string(req),
            JsonRpcMessage::Response(resp) => serde_json::to_string(resp),
            JsonRpcMessage::Notification(notif) => serde_json::to_string(notif),
        }
        .map_err(Into::into)
    }

    /// Convert an error to a JSON-RPC error
    pub fn error_to_jsonrpc(error: &Error) -> JsonRpcError {
        match error {
            Error::InvalidRequest(msg) => {
                Self::create_error(error_codes::INVALID_REQUEST, msg, None)
            }
            Error::MethodNotFound(method) => {
                Self::create_error(error_codes::METHOD_NOT_FOUND, method, None)
            }
            Error::InvalidParams(msg) => Self::create_error(error_codes::INVALID_PARAMS, msg, None),
            Error::Protocol(crate::error::ProtocolError::CapabilityNotSupported(cap)) => {
                Self::create_error(error_codes::CAPABILITY_NOT_SUPPORTED, cap, None)
            }
            Error::Protocol(crate::error::ProtocolError::ResourceNotFound(uri)) => {
                Self::create_error(error_codes::RESOURCE_NOT_FOUND, uri, None)
            }
            Error::Protocol(crate::error::ProtocolError::ToolNotFound(name)) => {
                Self::create_error(error_codes::TOOL_NOT_FOUND, name, None)
            }
            Error::Protocol(crate::error::ProtocolError::PromptNotFound(name)) => {
                Self::create_error(error_codes::PROMPT_NOT_FOUND, name, None)
            }
            Error::Protocol(crate::error::ProtocolError::PermissionDenied) => {
                Self::create_error(error_codes::PERMISSION_DENIED, "Permission denied", None)
            }
            Error::Protocol(crate::error::ProtocolError::RateLimitExceeded) => {
                Self::create_error(error_codes::RATE_LIMITED, "Rate limit exceeded", None)
            }
            Error::Parse(msg) => Self::create_error(error_codes::PARSE_ERROR, msg, None),
            _ => Self::create_error(error_codes::INTERNAL_ERROR, &error.to_string(), None),
        }
    }

    /// Validate method name format
    pub fn validate_method_name(method: &str) -> bool {
        !method.is_empty()
            && method
                .chars()
                .all(|c| c.is_alphanumeric() || c == '/' || c == '_')
    }

    /// Extract method category from method name (e.g., "tools/call" -> "tools")
    pub fn method_category(method: &str) -> Option<&str> {
        method.split('/').next()
    }
}

/// Unified JSON-RPC message type
#[derive(Debug, Clone)]
pub enum JsonRpcMessage {
    /// A JSON-RPC request message with a method to call and optional parameters
    Request(JsonRpcRequest),
    /// A JSON-RPC response message with a result or error
    Response(JsonRpcResponse),
    /// A JSON-RPC notification message (fire-and-forget)
    Notification(JsonRpcNotification),
}

impl JsonRpcMessage {
    /// Get the message ID if it exists
    pub fn id(&self) -> Option<&RequestId> {
        match self {
            JsonRpcMessage::Request(req) => req.id.as_ref(),
            JsonRpcMessage::Response(resp) => resp.id.as_ref(),
            JsonRpcMessage::Notification(_) => None,
        }
    }

    /// Get the method name if it exists
    pub fn method(&self) -> Option<&str> {
        match self {
            JsonRpcMessage::Request(req) => Some(&req.method),
            JsonRpcMessage::Response(_) => None,
            JsonRpcMessage::Notification(notif) => Some(&notif.method),
        }
    }

    /// Check if this is a request
    pub fn is_request(&self) -> bool {
        matches!(self, JsonRpcMessage::Request(_))
    }

    /// Check if this is a response
    pub fn is_response(&self) -> bool {
        matches!(self, JsonRpcMessage::Response(_))
    }

    /// Check if this is a notification
    pub fn is_notification(&self) -> bool {
        matches!(self, JsonRpcMessage::Notification(_))
    }
}
