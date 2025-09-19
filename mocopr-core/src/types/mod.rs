//! Core MCP types
//!
//! This module defines all the message types and data structures
//! specified in the Model Context Protocol specification.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
use uuid::Uuid;

pub mod capabilities;
pub mod messages;
pub mod prompts;
pub mod resources;
pub mod roots;
pub mod sampling;
pub mod tools;

pub use capabilities::*;
pub use messages::*;
pub use prompts::*;
pub use resources::*;
pub use roots::*;
pub use sampling::*;
pub use tools::*;

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC protocol version (should be "2.0")
    pub jsonrpc: String,
    /// Unique identifier for the request
    pub id: Option<RequestId>,
    /// Name of the method to be invoked
    pub method: String,
    /// Parameters to the method (can be by-position or by-name)
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC protocol version (should be "2.0")
    pub jsonrpc: String,
    /// Identifier matching the request this is a response to
    pub id: Option<RequestId>,
    /// Result of the method call (only present if the call succeeded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error information (only present if the call failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// JSON-RPC protocol version (should be "2.0")
    pub jsonrpc: String,
    /// Name of the method to be invoked
    pub method: String,
    /// Parameters to the method (can be by-position or by-name)
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code (standard JSON-RPC codes or MCP-specific codes)
    pub code: i32,
    /// Brief error message
    pub message: String,
    /// Additional error information (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Request ID can be string, number, or null
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum RequestId {
    /// String identifier
    String(String),
    /// Numeric identifier
    Number(i64),
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        RequestId::String(s)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        RequestId::String(s.to_string())
    }
}

impl From<i64> for RequestId {
    fn from(n: i64) -> Self {
        RequestId::Number(n)
    }
}

impl From<Uuid> for RequestId {
    fn from(uuid: Uuid) -> Self {
        RequestId::String(uuid.to_string())
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestId::String(s) => write!(f, "{s}"),
            RequestId::Number(n) => write!(f, "{n}"),
        }
    }
}

/// Progress token for tracking long-running operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProgressToken {
    /// String-based progress token
    String(String),
    /// Numeric progress token
    Number(i64),
}

impl From<String> for ProgressToken {
    fn from(s: String) -> Self {
        ProgressToken::String(s)
    }
}

impl From<&str> for ProgressToken {
    fn from(s: &str) -> Self {
        ProgressToken::String(s.to_string())
    }
}

impl From<i64> for ProgressToken {
    fn from(n: i64) -> Self {
        ProgressToken::Number(n)
    }
}

/// Annotation for providing additional context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    /// Type of annotation
    #[serde(rename = "type")]
    pub annotation_type: String,
    /// Annotation text content
    pub text: String,
    /// Target audience for the annotation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Audience>,
    /// Priority level of the annotation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>,
}

/// Audience for annotations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Audience {
    /// Human user
    User,
    /// AI assistant
    Assistant,
}

/// Cursor for pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    /// Cursor value for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Common pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    /// Cursor value for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Text content with optional annotations.
///
/// Represents a piece of text content that can be included in messages or resources.
/// The text is stored as a UTF-8 encoded string and can include optional
/// annotations that provide additional semantic information about specific
/// ranges of the text.
///
/// # MCP Specification Compliance
///
/// This type represents the "text" content type as defined in the MCP specification.
///
/// # Examples
///
/// Basic text content:
/// ```rust
/// use mocopr_core::types::TextContent;
///
/// let text = TextContent::new("Hello, world!");
/// assert_eq!(text.text, "Hello, world!");
/// assert!(text.annotations.is_none());
/// ```
///
/// Text content with annotations:
/// ```rust
/// use mocopr_core::types::{TextContent, Annotation};
///
/// let annotations = vec![
///     Annotation {
///         annotation_type: "highlight".to_string(),
///         text: "This is highlighted text".to_string(),
///         audience: None,
///         priority: None,
///     }
/// ];
///
/// let text = TextContent::with_annotations("Hello, world!", annotations);
/// assert_eq!(text.text, "Hello, world!");
/// assert!(text.annotations.is_some());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    // Removed content_type field to avoid serde tag conflict
    /// The actual text content as a UTF-8 encoded string.
    ///
    /// This field contains the textual data represented by this content object.
    pub text: String,

    /// Optional annotations that provide additional semantic information about specific
    /// ranges of the text.
    ///
    /// Annotations can mark spans of text with metadata such as formatting,
    /// semantic meaning, or references to other entities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Annotation>>,
}

impl TextContent {
    /// Creates a new text content instance
    ///
    /// # Arguments
    /// * `text` - The text content
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            annotations: None,
        }
    }

    /// Creates a new text content instance with annotations
    ///
    /// # Arguments
    /// * `text` - The text content
    /// * `annotations` - Annotations for the text
    pub fn with_annotations(text: impl Into<String>, annotations: Vec<Annotation>) -> Self {
        Self {
            text: text.into(),
            annotations: Some(annotations),
        }
    }
}

/// Image content for visual elements in messages or resources.
///
/// Represents an image that can be included in MCP messages or resources.
/// The image data is encoded as a base64 string and includes a MIME type
/// to specify the image format (e.g., "image/png", "image/jpeg").
///
/// # MCP Specification Compliance
///
/// This type represents the "image" content type as defined in the MCP specification.
///
/// # Examples
///
/// ```rust
/// use mocopr_core::types::ImageContent;
///
/// // In a real application, this would be actual base64-encoded image data
/// let base64_data = "R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7".to_string();
/// let image = ImageContent::new(base64_data, "image/gif");
///
/// assert_eq!(image.content_type, "image");
/// assert_eq!(image.mime_type, "image/gif");
/// assert!(image.annotations.is_none());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    /// Content type, always "image"
    #[serde(rename = "type")]
    pub content_type: String, // Always "image"
    /// Base64 encoded image data
    pub data: String, // Base64 encoded image data
    /// MIME type of the image
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// Optional annotations for the image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Annotation>>,
}

impl ImageContent {
    /// Creates a new image content instance
    ///
    /// # Arguments
    /// * `data` - Base64 encoded image data
    /// * `mime_type` - MIME type of the image
    pub fn new(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            content_type: "image".to_string(),
            data: data.into(),
            mime_type: mime_type.into(),
            annotations: None,
        }
    }
}

/// Content types that can be sent in messages or included in resources.
///
/// The MCP specification supports multiple content types to handle different
/// kinds of data. This enum represents the supported content types in the protocol.
///
/// # Content Types
///
/// - `Text`: Plain text content, potentially with annotations
/// - `Image`: Image content in base64-encoded format with a specific MIME type
///
/// # MCP Specification Compliance
///
/// This enum follows the MCP specification's content type model, where different
/// kinds of content are distinguished by a `type` field.
///
/// # Examples
///
/// Creating text content:
/// ```rust
/// use mocopr_core::types::{Content, TextContent};
///
/// // From a TextContent struct
/// let text_content = TextContent::new("Hello, world!");
/// let content = Content::from(text_content);
///
/// // From a String
/// let content = Content::from("Hello, world!".to_string());
///
/// // From a &str
/// let content = Content::from("Hello, world!");
/// ```
///
/// Creating image content:
/// ```rust
/// use mocopr_core::types::{Content, ImageContent};
///
/// let image_data = "base64encodedimagedata".to_string();
/// let image_content = ImageContent::new(image_data, "image/png");
/// let content = Content::from(image_content);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Content {
    /// Text content variant, containing UTF-8 encoded text and optional annotations.
    ///
    /// This is the most common content type for textual data, documentation,
    /// code snippets, and any other text-based information.
    #[serde(rename = "text")]
    Text(TextContent),

    /// Image content variant, containing base64-encoded image data, MIME type,
    /// and optional annotations.
    ///
    /// This content type is used for diagrams, photos, screenshots, and any
    /// other visual elements that need to be included in messages or resources.
    #[serde(rename = "image")]
    Image(ImageContent),

    /// Structured error content for providing detailed error information.
    ///
    /// This content type allows tools and resources to return structured
    /// error information instead of plain text, which can be more easily
    /// parsed and handled by clients.
    #[serde(rename = "error")]
    StructuredError(StructuredErrorContent),
}

/// Structured error content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredErrorContent {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Optional status code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
}

impl StructuredErrorContent {
    /// Creates a new structured error content instance
    ///
    /// # Arguments
    /// * `code` - The error code
    /// * `message` - The error message
    /// * `status` - Optional status code
    pub fn new(code: impl Into<String>, message: impl Into<String>, status: Option<u16>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            status,
        }
    }
}

impl From<TextContent> for Content {
    fn from(text: TextContent) -> Self {
        Content::Text(text)
    }
}

impl From<ImageContent> for Content {
    fn from(image: ImageContent) -> Self {
        Content::Image(image)
    }
}

impl From<String> for Content {
    fn from(text: String) -> Self {
        Content::Text(TextContent::new(text))
    }
}

impl From<&str> for Content {
    fn from(text: &str) -> Self {
        Content::Text(TextContent::new(text))
    }
}

/// Implementation info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    /// Name of the implementation
    pub name: String,
    /// Version of the implementation
    pub version: String,
}

/// Meta information included in responses
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseMetadata {
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<serde_json::Value>,
}

impl ResponseMetadata {
    /// Creates a new empty response metadata instance
    pub fn new() -> Self {
        Self::default()
    }
}

/// Generic result type for paginated responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    /// The actual data
    #[serde(flatten)]
    pub data: T,
    /// Cursor for the next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Response metadata
    #[serde(flatten)]
    pub meta: ResponseMetadata,
}
