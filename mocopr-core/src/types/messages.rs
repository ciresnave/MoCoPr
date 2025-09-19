//! MCP protocol messages
//!
//! Defines all the standard MCP messages for client-server communication/// Logging notification
/// Logging notification sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingNotification {
    /// The severity level of the log message
    pub level: LogLevel,
    /// The log message content or structured data
    pub data: serde_json::Value,
    /// Optional logger name/identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
}

/// Cancelled notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelledNotification {
    /// ID of the request being cancelled
    #[serde(rename = "requestId")]
    pub request_id: RequestId,
    /// Optional reason for cancellation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

use super::*;

/// Initialize request - first message sent by client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    /// The MCP protocol version supported by the client
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// The capabilities supported by the client
    pub capabilities: ClientCapabilities,
    /// Information about the client implementation
    #[serde(rename = "clientInfo")]
    pub client_info: Implementation,
}

/// Initialize response from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    /// The MCP protocol version that will be used for this session
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// The capabilities supported by the server
    pub capabilities: ServerCapabilities,
    /// Information about the server implementation
    #[serde(rename = "serverInfo")]
    pub server_info: Implementation,
    /// Optional instructions for the client (may include usage hints)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

/// Initialized notification - sent by client after receiving initialize response
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InitializedNotification {
    // No additional fields required
}

/// Ping request for connection health check
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PingRequest {
    /// Optional message to send with the ping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Ping response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResponse {
    /// Optional response message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Progress notification
/// Notification for operation progress updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressNotification {
    /// Token identifying the operation being tracked
    #[serde(rename = "progressToken")]
    pub progress_token: ProgressToken,
    /// Current progress value (e.g., 0.5 for 50%)
    pub progress: f64,
    /// Optional total value for progress calculation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
}

/// Log message levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Detailed debug information
    Debug,
    /// Interesting events
    Info,
    /// Normal but significant events
    Notice,
    /// Warning conditions
    Warning,
    /// Error conditions
    Error,
    /// Critical conditions
    Critical,
    /// Action must be taken immediately
    Alert,
    /// System is unusable
    Emergency,
}

/// Logging notification
// Note: These structs were defined earlier in the file
/// Unified message type for all MCP messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum McpMessage {
    // Core protocol
    /// Initialize the MCP session (client request)
    #[serde(rename = "initialize")]
    Initialize(InitializeRequest),
    /// Notification that the client has completed initialization
    #[serde(rename = "initialized")]
    Initialized(InitializedNotification),
    /// Ping/health check request
    #[serde(rename = "ping")]
    Ping(PingRequest),

    // Resources
    /// List available resources
    #[serde(rename = "resources/list")]
    ResourcesList(ResourcesListRequest),
    /// Read a specific resource
    #[serde(rename = "resources/read")]
    ResourcesRead(ResourcesReadRequest),
    /// Subscribe to a resource for updates
    #[serde(rename = "resources/subscribe")]
    ResourcesSubscribe(ResourcesSubscribeRequest),
    /// Unsubscribe from resource updates
    #[serde(rename = "resources/unsubscribe")]
    ResourcesUnsubscribe(ResourcesUnsubscribeRequest),

    // Tools
    /// List available tools
    #[serde(rename = "tools/list")]
    ToolsList(ToolsListRequest),
    /// Call a specific tool
    #[serde(rename = "tools/call")]
    ToolsCall(ToolsCallRequest),

    // Prompts
    /// List available prompts
    #[serde(rename = "prompts/list")]
    PromptsList(PromptsListRequest),
    /// Get a specific prompt
    #[serde(rename = "prompts/get")]
    PromptsGet(PromptsGetRequest),

    // Logging
    /// Set logging level
    #[serde(rename = "logging/setLevel")]
    LoggingSetLevel(LoggingSetLevelRequest),

    // Sampling (client capabilities)
    /// Create a message using the sampling capability
    #[serde(rename = "sampling/createMessage")]
    SamplingCreateMessage(CreateMessageRequest),

    // Roots (client capabilities)
    /// List available roots
    #[serde(rename = "roots/list")]
    RootsList(RootsListRequest),

    // Notifications
    /// Progress update notification
    #[serde(rename = "notifications/progress")]
    Progress(ProgressNotification),
    /// Log message notification
    #[serde(rename = "notifications/message")]
    LoggingMessage(LoggingNotification),
    /// Request cancellation notification
    #[serde(rename = "notifications/cancelled")]
    Cancelled(CancelledNotification),
    /// Resources list has been updated
    #[serde(rename = "notifications/resources/updated")]
    ResourcesUpdated(ResourcesUpdatedNotification),
    /// Tools list has been updated
    #[serde(rename = "notifications/tools/updated")]
    ToolsUpdated(ToolsListChangedNotification),
    /// Prompts list has been updated
    #[serde(rename = "notifications/prompts/updated")]
    PromptsUpdated(PromptsListChangedNotification),
    /// Roots list has been updated
    #[serde(rename = "notifications/roots/updated")]
    RootsUpdated(RootsListChangedNotification),
}

/// Unified response type for all MCP responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpResponse {
    /// Response to initialize request
    Initialize(InitializeResponse),
    /// Response to ping request
    Ping(PingResponse),
    /// Response to resources list request
    ResourcesList(ResourcesListResponse),
    /// Response to resources read request
    ResourcesRead(ResourcesReadResponse),
    /// Response to resources subscribe request
    ResourcesSubscribe(ResourcesSubscribeResponse),
    /// Response to resources unsubscribe request
    ResourcesUnsubscribe(ResourcesUnsubscribeResponse),
    /// Response to tools list request
    ToolsList(ToolsListResponse),
    /// Response to tools call request
    ToolsCall(ToolsCallResponse),
    /// Response to prompts list request
    PromptsList(PromptsListResponse),
    /// Response to prompts get request
    PromptsGet(PromptsGetResponse),
    /// Response to logging set level request
    LoggingSetLevel(LoggingSetLevelResponse),
    /// Response to sampling create message request
    SamplingCreateMessage(CreateMessageResponse),
    /// Response to roots list request
    RootsList(RootsListResponse),
    /// Empty response for notifications
    Empty(EmptyResponse),
}

/// Empty response for operations that don't return data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyResponse {
    /// Metadata for the response
    #[serde(flatten)]
    pub meta: ResponseMetadata,
}

impl EmptyResponse {
    /// Creates a new empty response with default metadata
    pub fn new() -> Self {
        Self {
            meta: ResponseMetadata { _meta: None },
        }
    }
}

impl Default for EmptyResponse {
    fn default() -> Self {
        Self::new()
    }
}
