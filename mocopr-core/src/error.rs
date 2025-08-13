//! Error types for MoCoPr.
//!
//! This module defines all error types used throughout the MoCoPr library.
//! The error types are organized hierarchically with a main `Error` enum
//! and specific error types for different subsystems like transport and protocol.
//!
//! # Error Handling Philosophy
//!
//! MoCoPr uses structured error types to provide meaningful error information
//! while maintaining compatibility with the JSON-RPC 2.0 error format used
//! by the Model Context Protocol.
//!
//! # Examples
//!
//! ```rust
//! use mocopr_core::error::{Error, Result, ProtocolError};
//!
//! fn example_function() -> Result<String> {
//!     Err(Error::Protocol(ProtocolError::ToolNotFound("test_tool".to_string())))
//! }
//! ```

use thiserror::Error;

/// Result type alias for MoCoPr operations.
///
/// This is a convenience type alias that uses the MoCoPr `Error` type
/// as the error variant. Use this for all functions that can return
/// MoCoPr-specific errors.
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for MoCoPr operations.
///
/// This enum covers all possible errors that can occur during MCP operations,
/// from transport-level failures to protocol violations and application-level errors.
/// Each error variant provides specific context about what went wrong.
///
/// # JSON-RPC Error Mapping
///
/// These errors can be mapped to JSON-RPC 2.0 error codes when sent over the wire:
/// - `InvalidRequest` → -32600
/// - `MethodNotFound` → -32601
/// - `InvalidParams` → -32602
/// - `Internal` → -32603
/// - `Parse` → -32700
///
/// # Examples
///
/// ```rust
/// use mocopr_core::error::{Error, ProtocolError};
///
/// // Create different types of errors
/// let transport_err = Error::ConnectionClosed;
/// let protocol_err = Error::Protocol(ProtocolError::ToolNotFound("my_tool".to_string()));
/// let validation_err = Error::InvalidParams("Missing required parameter 'path'".to_string());
/// ```
#[derive(Debug, Error)]
pub enum Error {
    /// Transport layer error (connection, send/receive failures, etc.).
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    /// Protocol layer error (capability negotiation, message sequencing, etc.).
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    /// JSON serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// The request is malformed or violates the protocol specification.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// The requested method/operation is not supported.
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    /// The provided parameters are invalid or missing required fields.
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    /// An internal server error occurred.
    #[error("Internal error: {0}")]
    Internal(String),

    /// The operation was cancelled by the user or system.
    #[error("Request cancelled")]
    Cancelled,

    /// Failed to parse message or data format.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Server-side error during request processing.
    #[error("Server error: {0}")]
    Server(String),

    /// Client-side error in request formation or handling.
    #[error("Client error: {0}")]
    Client(String),

    /// Operation timed out.
    #[error("Timeout")]
    Timeout,

    /// The connection was closed unexpectedly.
    #[error("Connection closed")]
    ConnectionClosed,

    /// Input/output error from the underlying system.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// URL parsing failed.
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    /// Security-related error (authentication, authorization, validation).
    #[error("Security error: {0}")]
    Security(String),

    /// Configuration error (invalid settings, missing config, etc.).
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Resource access error (file not found, permission denied, etc.).
    #[error("Resource access error: {0}")]
    ResourceAccess(String),

    /// Validation error (schema validation, constraint violation, etc.).
    #[error("Validation error: {0}")]
    Validation(String),

    /// Catch-all for other error types.
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

/// Transport-specific errors.
///
/// These errors occur at the transport layer and relate to the underlying
/// communication mechanism (stdio, websockets, HTTP, etc.).
///
/// # Examples
///
/// ```rust
/// use mocopr_core::error::TransportError;
///
/// let error = TransportError::ConnectionFailed("Unable to connect to server".to_string());
/// ```
#[derive(Debug, Error)]
pub enum TransportError {
    /// Failed to establish a connection to the remote endpoint.
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Failed to send a message through the transport.
    #[error("Send failed: {0}")]
    SendFailed(String),

    /// Failed to receive a message from the transport.
    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    /// The message format is invalid for this transport.
    #[error("Invalid message format")]
    InvalidMessageFormat,

    /// The transport is not ready for operations.
    #[error("Transport not ready")]
    NotReady,

    /// The transport has been closed and cannot be used.
    #[error("Transport closed")]
    Closed,

    /// Authentication failed with the transport layer.
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// The transport configuration is invalid.
    #[error("Invalid transport configuration: {0}")]
    InvalidConfiguration(String),

    /// Network error occurred during transport operations.
    #[error("Network error: {0}")]
    NetworkError(String),
}

/// Protocol-specific errors.
///
/// These errors occur at the MCP protocol layer and relate to protocol
/// violations, capability mismatches, or invalid message sequences.
///
/// # Examples
///
/// ```rust
/// use mocopr_core::error::ProtocolError;
///
/// let error = ProtocolError::ToolNotFound("nonexistent_tool".to_string());
/// ```
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// A required capability is not supported by the remote endpoint.
    #[error("Capability not supported: {0}")]
    CapabilityNotSupported(String),

    /// The capability negotiation process failed or was invalid.
    #[error("Invalid capability negotiation")]
    InvalidCapabilityNegotiation,

    /// Received an unexpected message type for the current protocol state.
    #[error("Unexpected message type")]
    UnexpectedMessageType,

    /// The message sequence violates the protocol specification.
    #[error("Invalid message sequence")]
    InvalidMessageSequence,

    /// The requested resource was not found on the server.
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    /// The requested tool was not found on the server.
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// The requested prompt was not found on the server.
    #[error("Prompt not found: {0}")]
    PromptNotFound(String),

    /// Access to the requested resource or operation was denied.
    #[error("Permission denied")]
    PermissionDenied,

    /// The rate limit for requests has been exceeded.
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// The protocol version is not supported.
    #[error("Unsupported protocol version: {0}")]
    UnsupportedProtocolVersion(String),

    /// The initialization handshake failed.
    #[error("Initialization failed: {0}")]
    InitializationFailed(String),

    /// The session is in an invalid state for the requested operation.
    #[error("Invalid session state: {0}")]
    InvalidSessionState(String),

    /// A required parameter is missing from the request.
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),

    /// A parameter value is out of the valid range or format.
    #[error("Invalid parameter value: {0}")]
    InvalidParameterValue(String),
}

impl Error {
    /// Create a new internal error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::error::Error;
    ///
    /// let error = Error::internal("Something went wrong internally");
    /// ```
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Create a new transport error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::error::{Error, TransportError};
    ///
    /// let error = Error::transport(TransportError::ConnectionFailed("Network unreachable".to_string()));
    /// ```
    pub fn transport(err: TransportError) -> Self {
        Self::Transport(err)
    }

    /// Create a new protocol error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::error::{Error, ProtocolError};
    ///
    /// let error = Error::protocol(ProtocolError::ToolNotFound("missing_tool".to_string()));
    /// ```
    pub fn protocol(err: ProtocolError) -> Self {
        Self::Protocol(err)
    }

    /// Create a new security error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::error::Error;
    ///
    /// let error = Error::security("Invalid authentication token");
    /// ```
    pub fn security(msg: impl Into<String>) -> Self {
        Self::Security(msg.into())
    }

    /// Create a new validation error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::error::Error;
    ///
    /// let error = Error::validation("Parameter 'path' must be an absolute path");
    /// ```
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a new resource access error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::error::Error;
    ///
    /// let error = Error::resource_access("File not found: /path/to/file.txt");
    /// ```
    pub fn resource_access(msg: impl Into<String>) -> Self {
        Self::ResourceAccess(msg.into())
    }

    /// Create a new method not found error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::error::Error;
    ///
    /// let error = Error::method_not_found("nonexistent_method");
    /// ```
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self::MethodNotFound(method.into())
    }

    /// Create a new invalid params error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::error::Error;
    ///
    /// let error = Error::invalid_params("Missing required parameter 'path'");
    /// ```
    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self::InvalidParams(msg.into())
    }

    /// Create a new not found error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::error::Error;
    ///
    /// let error = Error::not_found("Resource not found");
    /// ```
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::ResourceAccess(msg.into())
    }

    /// Create a new operation failed error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::error::Error;
    ///
    /// let error = Error::operation_failed("Operation failed after multiple attempts");
    /// ```
    pub fn operation_failed(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Create a new resource error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::error::Error;
    ///
    /// let error = Error::resource_error("Failed to read file");
    /// ```
    pub fn resource_error(msg: impl Into<String>) -> Self {
        Self::ResourceAccess(msg.into())
    }

    /// Check if the error is recoverable.
    ///
    /// Recoverable errors are those that might succeed if retried,
    /// while non-recoverable errors indicate permanent failures.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::error::{Error, TransportError};
    ///
    /// let timeout = Error::Timeout;
    /// assert!(timeout.is_recoverable());
    ///
    /// let closed = Error::ConnectionClosed;
    /// assert!(!closed.is_recoverable());
    /// ```
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Transport(TransportError::NotReady) => true,
            Self::Transport(TransportError::Closed) => false,
            Self::Transport(TransportError::NetworkError(_)) => true,
            Self::ConnectionClosed => false,
            Self::Cancelled => false,
            Self::Timeout => true,
            Self::Security(_) => false,
            Self::Configuration(_) => false,
            Self::Protocol(ProtocolError::RateLimitExceeded) => true,
            _ => true,
        }
    }

    /// Get the JSON-RPC error code for this error.
    ///
    /// Maps MoCoPr errors to standard JSON-RPC 2.0 error codes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::error::Error;
    ///
    /// let error = Error::InvalidRequest("Malformed JSON".to_string());
    /// assert_eq!(error.json_rpc_code(), -32600);
    /// ```
    pub fn json_rpc_code(&self) -> i32 {
        match self {
            Self::Parse(_) => -32700,
            Self::InvalidRequest(_) => -32600,
            Self::MethodNotFound(_) => -32601,
            Self::InvalidParams(_) | Self::Validation(_) => -32602,
            Self::Internal(_) => -32603,
            Self::Protocol(ProtocolError::ToolNotFound(_)) => -32601,
            Self::Protocol(ProtocolError::ResourceNotFound(_)) => -32601,
            Self::Protocol(ProtocolError::PromptNotFound(_)) => -32601,
            Self::Security(_) | Self::Protocol(ProtocolError::PermissionDenied) => -32000,
            Self::Protocol(ProtocolError::RateLimitExceeded) => -32001,
            Self::Timeout => -32002,
            Self::ConnectionClosed => -32003,
            _ => -32000, // Generic server error
        }
    }

    /// Check if this is a client-side error.
    ///
    /// Client errors are those caused by invalid requests or client configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::error::Error;
    ///
    /// let client_error = Error::InvalidParams("Missing parameter".to_string());
    /// assert!(client_error.is_client_error());
    ///
    /// let server_error = Error::Internal("Database connection failed".to_string());
    /// assert!(!server_error.is_client_error());
    /// ```
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidRequest(_)
                | Self::MethodNotFound(_)
                | Self::InvalidParams(_)
                | Self::Parse(_)
                | Self::Client(_)
                | Self::Validation(_)
                | Self::UrlParse(_)
                | Self::Protocol(ProtocolError::UnsupportedProtocolVersion(_))
                | Self::Protocol(ProtocolError::MissingParameter(_))
                | Self::Protocol(ProtocolError::InvalidParameterValue(_))
        )
    }
}
