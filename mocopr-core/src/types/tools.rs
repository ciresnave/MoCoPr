//! Tool-related types and messages for the Model Context Protocol.
//!
//! This module defines all the types and structures needed to work with MCP tools.
//! Tools are functions that can be called by AI assistants to perform specific tasks,
//! such as reading files, making API calls, or executing commands.
//!
//! # Example
//!
//! ```rust
//! use mocopr_core::types::Tool;
//! use serde_json::json;
//!
//! let tool = Tool {
//!     name: "read_file".to_string(),
//!     description: Some("Read the contents of a file".to_string()),
//!     input_schema: json!({
//!         "type": "object",
//!         "properties": {
//!             "path": {
//!                 "type": "string",
//!                 "description": "Path to the file to read"
//!             }
//!         },
//!         "required": ["path"]
//!     })
//! };
//! ```

use super::*;
use smallvec::SmallVec;

/// Tool represents a function that can be called by the AI.
///
/// Tools are the primary way for AI assistants to interact with external systems
/// through the Model Context Protocol. Each tool defines its interface through
/// a JSON schema that describes the expected input parameters.
///
/// # Examples
///
/// ```rust
/// use mocopr_core::types::Tool;
/// use serde_json::json;
///
/// // A simple tool that takes a string parameter
/// let echo_tool = Tool {
///     name: "echo".to_string(),
///     description: Some("Echo back the input text".to_string()),
///     input_schema: json!({
///         "type": "object",
///         "properties": {
///             "text": {
///                 "type": "string",
///                 "description": "Text to echo back"
///             }
///         },
///         "required": ["text"]
///     })
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// The unique name of the tool. Must be a valid identifier.
    pub name: String,
    /// Optional human-readable description of what the tool does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema defining the structure of the tool's input parameters.
    /// This should be a valid JSON Schema object that describes the expected
    /// arguments for the tool.
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// Tool parameter definition for schema validation.
///
/// This type represents a single parameter in a tool's input schema.
/// It provides metadata about the parameter including its type, description,
/// whether it's required, default values, and examples.
///
/// # Examples
///
/// ```rust
/// use mocopr_core::types::ToolParameter;
/// use serde_json::json;
///
/// let param = ToolParameter {
///     param_type: "string".to_string(),
///     description: Some("The file path to read".to_string()),
///     required: Some(true),
///     default: None,
///     examples: Some(vec![json!("/path/to/file.txt")]),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// The JSON Schema type of this parameter (e.g., "string", "number", "object").
    #[serde(rename = "type")]
    pub param_type: String,
    /// Optional human-readable description of this parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this parameter is required. If None, assumes false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    /// Default value for this parameter if not provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Example values for this parameter to help users understand expected input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<serde_json::Value>>,
}

/// Request to list available tools from the server.
///
/// This message is sent by clients to discover what tools are available
/// on the server. The server responds with a list of tool definitions.
///
/// # Examples
///
/// ```rust
/// use mocopr_core::types::{ToolsListRequest, PaginationParams};
///
/// let request = ToolsListRequest {
///     pagination: PaginationParams {
///         cursor: None,
///     }
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListRequest {
    /// Pagination parameters to control the number of tools returned
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// Response to list tools request.
///
/// Contains the list of available tools and optional pagination information
/// for handling large tool sets.
///
/// # Examples
///
/// ```rust
/// use mocopr_core::types::{ToolsListResponse, Tool, ResponseMetadata};
/// use serde_json::json;
///
/// let response = ToolsListResponse {
///     tools: vec![
///         Tool {
///             name: "echo".to_string(),
///             description: Some("Echo text".to_string()),
///             input_schema: json!({"type": "object"}),
///         }
///     ],
///     next_cursor: None,
///     meta: ResponseMetadata::default(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListResponse {
    /// List of available tools.
    pub tools: Vec<Tool>,
    /// Cursor for pagination. Present if there are more tools to fetch.
    #[serde(rename = "nextCursor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Response metadata including protocol version and other information.
    #[serde(flatten)]
    pub meta: ResponseMetadata,
}

/// Request to call a tool with specific arguments.
///
/// This message is sent by clients to invoke a tool on the server.
/// The server will execute the tool and return the results.
///
/// # Examples
///
/// ```rust
/// use mocopr_core::types::ToolsCallRequest;
/// use serde_json::json;
///
/// let request = ToolsCallRequest {
///     name: "read_file".to_string(),
///     arguments: Some(json!({
///         "path": "/path/to/file.txt"
///     })),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCallRequest {
    /// The name of the tool to call.
    pub name: String,
    /// Arguments to pass to the tool, structured according to the tool's input schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// Response to tool call request.
///
/// Contains the results of the tool execution, including any content
/// produced by the tool and error information if the call failed.
///
/// # Performance Note
///
/// The `content` field uses `SmallVec` for performance optimization since
/// most tool responses contain only 1-2 content items.
///
/// # Examples
///
/// ```rust
/// use mocopr_core::types::{ToolsCallResponse, Content, ResponseMetadata, TextContent};
///
/// let response = ToolsCallResponse {
///     content: vec![
///         Content::Text(TextContent::new("File contents here"))
///     ].into(),
///     is_error: Some(false),
///     meta: ResponseMetadata::default(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCallResponse {
    /// Content produced by the tool execution.
    /// Optimized for 1-2 items (typical case) using SmallVec.
    pub content: SmallVec<[Content; 2]>,
    /// Whether the tool execution resulted in an error.
    #[serde(rename = "isError")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    /// Response metadata including protocol version and other information.
    #[serde(flatten)]
    pub meta: ResponseMetadata,
}

/// Notification that tools list has changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListChangedNotification {
    // No additional fields required
}

/// Logging configuration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSetLevelRequest {
    /// The log level to set for the server
    pub level: LogLevel,
}

/// Logging configuration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSetLevelResponse {
    /// Metadata for the response
    #[serde(flatten)]
    pub meta: ResponseMetadata,
}

impl Tool {
    /// Creates a new tool with the given name and input schema
    ///
    /// # Arguments
    /// * `name` - The name of the tool
    /// * `input_schema` - JSON schema describing the tool's input parameters
    pub fn new(name: impl Into<String>, input_schema: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            description: None,
            input_schema,
        }
    }

    /// Adds a description to the tool
    ///
    /// # Arguments
    /// * `description` - Human-readable description of what the tool does
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl ToolParameter {
    /// Creates a new tool parameter with the given type
    ///
    /// # Arguments
    /// * `param_type` - The type of the parameter (e.g., "string", "number", "object")
    pub fn new(param_type: impl Into<String>) -> Self {
        Self {
            param_type: param_type.into(),
            description: None,
            required: None,
            default: None,
            examples: None,
        }
    }

    /// Adds a description to the parameter
    ///
    /// # Arguments
    /// * `description` - Human-readable description of what the parameter is for
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Marks the parameter as required
    pub fn required(mut self) -> Self {
        self.required = Some(true);
        self
    }

    /// Marks the parameter as optional (not required)
    pub fn optional(mut self) -> Self {
        self.required = Some(false);
        self
    }

    /// Sets a default value for the parameter
    ///
    /// # Arguments
    /// * `default` - The default value to use if the parameter is not provided
    pub fn with_default(mut self, default: serde_json::Value) -> Self {
        self.default = Some(default);
        self
    }
}

impl ToolsListRequest {
    /// Creates a new tools list request with default pagination
    pub fn new() -> Self {
        Self {
            pagination: PaginationParams { cursor: None },
        }
    }

    /// Sets a cursor for pagination
    ///
    /// # Arguments
    /// * `cursor` - The cursor value from a previous request to continue pagination
    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.pagination.cursor = Some(cursor.into());
        self
    }
}

impl Default for ToolsListRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolsCallRequest {
    /// Creates a new tool call request for the given tool
    ///
    /// # Arguments
    /// * `name` - The name of the tool to call
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: None,
        }
    }

    /// Sets the arguments for the tool call
    ///
    /// # Arguments
    /// * `arguments` - The arguments to pass to the tool as a JSON value
    pub fn with_arguments(mut self, arguments: serde_json::Value) -> Self {
        self.arguments = Some(arguments);
        self
    }
}

impl ToolsCallResponse {
    /// Create a successful tool response with content.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::types::{ToolsCallResponse, Content, TextContent};
    ///
    /// let response = ToolsCallResponse::success(vec![
    ///     Content::Text(TextContent::new("Success!"))
    /// ]);
    /// ```
    pub fn success(content: Vec<Content>) -> Self {
        Self {
            content: content.into(),
            is_error: Some(false),
            meta: ResponseMetadata { _meta: None },
        }
    }

    /// Create an error tool response with content.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::types::{ToolsCallResponse, Content, TextContent};
    ///
    /// let response = ToolsCallResponse::error(vec![
    ///     Content::Text(TextContent::new("Error occurred"))
    /// ]);
    /// ```
    pub fn error(content: Vec<Content>) -> Self {
        Self {
            content: content.into(),
            is_error: Some(true),
            meta: ResponseMetadata { _meta: None },
        }
    }

    /// Create a successful tool response with a single content item.
    ///
    /// This is a convenience method for the common case of returning
    /// a single piece of content.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::types::{ToolsCallResponse, Content, TextContent};
    ///
    /// let response = ToolsCallResponse::success_single(
    ///     Content::Text(TextContent::new("Single result"))
    /// );
    /// ```
    pub fn success_single(content: Content) -> Self {
        let mut result = SmallVec::new();
        result.push(content);
        Self {
            content: result,
            is_error: Some(false),
            meta: ResponseMetadata { _meta: None },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_parameter_creation() {
        let param = ToolParameter {
            param_type: "string".to_string(),
            description: Some("A test parameter".to_string()),
            required: Some(true),
            default: None,
            examples: Some(vec![json!("example_value")]),
        };

        assert_eq!(param.param_type, "string");
        assert_eq!(param.description, Some("A test parameter".to_string()));
        assert_eq!(param.required, Some(true));
        assert!(param.default.is_none());
        assert_eq!(param.examples, Some(vec![json!("example_value")]));
    }

    #[test]
    fn test_tool_parameter_serialization() {
        let param = ToolParameter {
            param_type: "number".to_string(),
            description: Some("A numeric parameter".to_string()),
            required: Some(false),
            default: Some(json!(42)),
            examples: None,
        };

        let serialized = serde_json::to_string(&param).unwrap();
        let deserialized: ToolParameter = serde_json::from_str(&serialized).unwrap();

        assert_eq!(param.param_type, deserialized.param_type);
        assert_eq!(param.description, deserialized.description);
        assert_eq!(param.required, deserialized.required);
        assert_eq!(param.default, deserialized.default);
        assert_eq!(param.examples, deserialized.examples);
    }

    #[test]
    fn test_tool_creation() {
        let tool = Tool {
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "param1": {
                        "type": "string",
                        "description": "First parameter"
                    }
                },
                "required": ["param1"]
            }),
        };

        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, Some("A test tool".to_string()));
        assert!(tool.input_schema.is_object());
    }

    #[test]
    fn test_tool_serialization() {
        let tool = Tool {
            name: "calculator".to_string(),
            description: Some("A simple calculator".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "operation": {"type": "string"},
                    "a": {"type": "number"},
                    "b": {"type": "number"}
                },
                "required": ["operation", "a", "b"]
            }),
        };

        let serialized = serde_json::to_string(&tool).unwrap();
        let deserialized: Tool = serde_json::from_str(&serialized).unwrap();

        assert_eq!(tool.name, deserialized.name);
        assert_eq!(tool.description, deserialized.description);
        assert_eq!(tool.input_schema, deserialized.input_schema);
    }

    #[test]
    fn test_tool_parameter_json_compliance() {
        let param = ToolParameter {
            param_type: "object".to_string(),
            description: Some("An object parameter".to_string()),
            required: Some(true),
            default: None,
            examples: None,
        };

        let json_val = serde_json::to_value(&param).unwrap();

        // Check that "type" field is properly renamed
        assert_eq!(json_val["type"], "object");
        assert_eq!(json_val["description"], "An object parameter");
        assert_eq!(json_val["required"], true);

        // Check that None fields are not serialized
        assert!(json_val.get("default").is_none());
        assert!(json_val.get("examples").is_none());
    }
}
