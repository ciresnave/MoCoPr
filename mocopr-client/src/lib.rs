//! # MoCoPr Client
//!
//! A high-level MCP client implementation for connecting to MCP servers.
//!
//! This crate provides a convenient, async API for building MCP clients that can
//! connect to any MCP server and perform operations like listing tools/resources,
//! calling tools, reading resources, and more.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use mocopr_client::McpClient;
//! use mocopr_core::prelude::*;
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Connect to an MCP server via stdio
//!     let client = McpClient::connect_stdio(
//!         "python",
//!         &["server.py"],
//!         Implementation {
//!             name: "My Client".to_string(),
//!             version: "1.0.0".to_string(),
//!         },
//!         ClientCapabilities::default(),
//!     ).await?;
//!
//!     // List available tools
//!     let tools = client.list_tools().await?;
//!     println!("Available tools: {:?}", tools);
//!
//!     // Call a tool
//!     let result = client.call_tool("example_tool".to_string(), Some(json!({}))).await?;
//!     println!("Tool result: {:?}", result);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Transport Support
//!
//! The client supports multiple transport mechanisms:
//! - **Stdio**: For process-based communication with MCP servers
//! - **WebSocket**: For real-time communication over the network
//! - **HTTP**: For stateless request/response interactions
//!
//! ## Features
//!
//! - Async/await based API
//! - Multiple transport support
//! - Automatic capability negotiation
//! - Type-safe API for MCP operations
//! - Built-in error handling and retry logic
//! - Comprehensive logging and debugging support

use mocopr_core::prelude::*;
use mocopr_core::transport::{TransportConfig, TransportFactory};
use std::sync::Arc;

/// High-level MCP client for connecting to and interacting with MCP servers.
///
/// The `McpClient` provides a convenient async API for performing MCP operations
/// like listing tools/resources, calling tools, reading resources, and handling prompts.
/// It automatically handles the MCP protocol details, capability negotiation, and
/// transport layer management.
///
/// # Examples
///
/// ## Connecting via stdio
///
/// ```rust,no_run
/// use mocopr_client::McpClient;
/// use mocopr_core::prelude::*;
///
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// let client = McpClient::connect_stdio(
///     "python",
///     &["server.py"],
///     Implementation {
///         name: "My Client".to_string(),
///         version: "1.0.0".to_string(),
///     },
///     ClientCapabilities::default(),
/// ).await?;
/// # Ok(())
/// # }
/// ```
///
/// ## Using tools
///
/// ```rust,no_run
/// # use mocopr_client::McpClient;
/// # use mocopr_core::prelude::*;
/// # use serde_json::json;
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// # let client = McpClient::connect_stdio("python", &["server.py"],
/// #     Implementation { name: "My Client".to_string(), version: "1.0.0".to_string() },
/// #     ClientCapabilities::default()).await?;
/// // List available tools
/// let tools = client.list_tools().await?;
/// for tool in &tools.tools {
///     println!("Tool: {} - {}", tool.name, tool.description.as_deref().unwrap_or(""));
/// }
///
/// // Call a specific tool
/// let result = client.call_tool("calculate".to_string(), Some(json!({
///     "expression": "2 + 2"
/// }))).await?;
/// println!("Calculation result: {:?}", result.content);
/// # Ok(())
/// # }
/// ```
pub struct McpClient {
    session: Arc<Session>,
    info: Implementation,
    capabilities: ClientCapabilities,
}

impl McpClient {
    /// Create a new MCP client with custom transport configuration.
    ///
    /// This method provides the most flexibility for creating a client with
    /// custom transport settings. For common use cases, consider using the
    /// convenience methods like [`McpClient::connect_stdio`] or [`McpClient::connect_websocket`].
    ///
    /// # Arguments
    ///
    /// * `transport_config` - Transport configuration (stdio, websocket, or HTTP)
    /// * `client_info` - Information about this client implementation
    /// * `client_capabilities` - Capabilities this client supports
    ///
    /// # Returns
    ///
    /// Returns a `Result<McpClient>` which is `Ok` if the connection was
    /// successful and capability negotiation completed.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - The transport connection fails
    /// - The server doesn't support MCP protocol
    /// - Capability negotiation fails
    /// - The server returns invalid responses
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use mocopr_client::McpClient;
    /// use mocopr_core::prelude::*;
    /// use mocopr_core::transport::TransportConfig;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// // Note: TransportConfig::Stdio creates a basic stdio transport
    /// // For connecting to specific commands, use connect_stdio() instead
    /// let config = TransportConfig::Stdio;
    ///
    /// let client = McpClient::new(
    ///     config,
    ///     Implementation {
    ///         name: "My Client".to_string(),
    ///         version: "1.0.0".to_string(),
    ///     },
    ///     ClientCapabilities::default(),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        transport_config: TransportConfig,
        client_info: Implementation,
        client_capabilities: ClientCapabilities,
    ) -> Result<Self> {
        let transport = TransportFactory::create(transport_config).await?;
        let handler = Arc::new(DefaultMessageHandler::new(
            Implementation {
                name: "MoCoPr Client".to_string(),
                version: "1.0.0".to_string(),
            },
            ServerCapabilities::default(),
        ));

        let (session, _events) = Session::new(transport, handler);
        let session = Arc::new(session);

        // Initialize the session
        session
            .initialize(client_info.clone(), client_capabilities.clone())
            .await?;

        Ok(Self {
            session,
            info: client_info,
            capabilities: client_capabilities,
        })
    }

    /// Connect to an MCP server via stdio (process communication).
    ///
    /// This is a convenience method for connecting to MCP servers that run as
    /// separate processes and communicate via stdin/stdout. This is one of the
    /// most common ways to connect to MCP servers.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute (e.g., "python", "node", "./my-server")
    /// * `args` - Command line arguments to pass to the command
    /// * `client_info` - Information about this client implementation
    /// * `client_capabilities` - Capabilities this client supports
    ///
    /// # Returns
    ///
    /// Returns a `Result<McpClient>` which is `Ok` if the server process was
    /// started successfully and the MCP handshake completed.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - The command cannot be executed (not found, permissions, etc.)
    /// - The server process fails to start or crashes immediately
    /// - The server doesn't implement MCP protocol correctly
    /// - Capability negotiation fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use mocopr_client::McpClient;
    /// use mocopr_core::prelude::*;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// // Connect to a Python MCP server
    /// let client = McpClient::connect_stdio(
    ///     "python",
    ///     &["path/to/server.py"],
    ///     Implementation {
    ///         name: "My Python Client".to_string(),
    ///         version: "1.0.0".to_string(),
    ///     },
    ///     ClientCapabilities::default(),
    /// ).await?;
    ///
    /// // Connect to a Node.js MCP server
    /// let client = McpClient::connect_stdio(
    ///     "node",
    ///     &["server.js"],
    ///     Implementation {
    ///         name: "My Node Client".to_string(),
    ///         version: "1.0.0".to_string(),
    ///     },
    ///     ClientCapabilities::default(),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_stdio(
        command: &str,
        args: &[&str],
        client_info: Implementation,
        capabilities: ClientCapabilities,
    ) -> Result<Self> {
        let transport = mocopr_core::transport::stdio::StdioTransport::spawn(command, args).await?;

        let handler = Arc::new(DefaultMessageHandler::new(
            Implementation {
                name: "MoCoPr Client".to_string(),
                version: "1.0.0".to_string(),
            },
            ServerCapabilities::default(),
        ));

        let (session, _events) = Session::new(Box::new(transport), handler);
        let session = Arc::new(session);

        // Initialize the session
        session
            .initialize(client_info.clone(), capabilities.clone())
            .await?;

        Ok(Self {
            session,
            info: client_info,
            capabilities,
        })
    }

    /// Connect to an MCP server via WebSocket
    ///
    /// This is a convenience method for connecting to MCP servers over the network
    /// using the WebSocket protocol. This is useful for real-time communication
    /// with MCP servers.
    ///
    /// # Arguments
    ///
    /// * `url` - The WebSocket URL of the MCP server (e.g., "ws://localhost:8080")
    /// * `client_info` - Information about this client implementation
    /// * `client_capabilities` - Capabilities this client supports
    ///
    /// # Returns
    ///
    /// Returns a `Result<McpClient>` which is `Ok` if the WebSocket connection
    /// was established successfully and the MCP handshake completed.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - The WebSocket connection fails (invalid URL, server not reachable, etc.)
    /// - The server doesn't implement MCP protocol correctly
    /// - Capability negotiation fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use mocopr_client::McpClient;
    /// use mocopr_core::prelude::*;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let client = McpClient::connect_websocket(
    ///     "ws://localhost:8080",
    ///     Implementation {
    ///         name: "My Client".to_string(),
    ///         version: "1.0.0".to_string(),
    ///     },
    ///     ClientCapabilities::default(),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_websocket(
        url: &str,
        client_info: Implementation,
        capabilities: ClientCapabilities,
    ) -> Result<Self> {
        let transport = mocopr_core::transport::websocket::WebSocketTransport::new(url).await?;

        let handler = Arc::new(DefaultMessageHandler::new(
            Implementation {
                name: "MoCoPr Client".to_string(),
                version: "1.0.0".to_string(),
            },
            ServerCapabilities::default(),
        ));

        let (session, _events) = Session::new(Box::new(transport), handler);
        let session = Arc::new(session);

        // Initialize the session
        session
            .initialize(client_info.clone(), capabilities.clone())
            .await?;

        Ok(Self {
            session,
            info: client_info,
            capabilities,
        })
    }

    /// List available resources
    ///
    /// This method sends a request to the server to list all available resources
    /// that the client can access. Resources are identified by URIs.
    ///
    /// # Returns
    ///
    /// Returns a `Result<ResourcesListResponse>` containing the list of resources.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - The server returns an error response
    /// - The response is missing required fields
    /// - JSON deserialization of the response fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use mocopr_client::McpClient;
    /// # use mocopr_core::prelude::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let client = McpClient::connect_stdio("python", &["server.py"],
    ///     Implementation { name: "My Client".to_string(), version: "1.0.0".to_string() },
    ///     ClientCapabilities::default()).await?;
    ///
    /// // List resources
    /// let resources = client.list_resources().await?;
    /// for resource in resources.resources {
    ///     println!("Resource URI: {}", resource.uri);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_resources(&self) -> Result<ResourcesListResponse> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Protocol::generate_request_id()),
            method: "resources/list".to_string(),
            params: Some(serde_json::to_value(&ResourcesListRequest::new())?),
        };

        let response = self.session.send_request(request).await?;
        if let Some(error) = response.error {
            return Err(Error::Server(error.message));
        }

        let result = response
            .result
            .ok_or_else(|| Error::Server("Missing result in response".to_string()))?;

        Ok(serde_json::from_value(result)?)
    }

    /// Read a resource
    ///
    /// This method reads the content of a specific resource identified by its URI.
    /// The resource must be of a type that the client knows how to handle.
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI of the resource to read
    ///
    /// # Returns
    ///
    /// Returns a `Result<ResourcesReadResponse>` containing the resource content.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - The server returns an error response
    /// - The response is missing required fields
    /// - JSON deserialization of the response fails
    /// - The resource type is not supported by the client
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use mocopr_client::McpClient;
    /// # use mocopr_core::prelude::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let client = McpClient::connect_stdio("python", &["server.py"],
    ///     Implementation { name: "My Client".to_string(), version: "1.0.0".to_string() },
    ///     ClientCapabilities::default()).await?;
    ///
    /// // Read a text resource
    /// let response = client.read_resource("file:///path/to/resource.txt".parse().unwrap()).await?;
    /// println!("Resource content: {:?}", response.contents);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn read_resource(&self, uri: url::Url) -> Result<ResourcesReadResponse> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Protocol::generate_request_id()),
            method: "resources/read".to_string(),
            params: Some(serde_json::to_value(&ResourcesReadRequest { uri })?),
        };

        let response = self.session.send_request(request).await?;
        if let Some(error) = response.error {
            return Err(Error::Server(error.message));
        }

        let result = response
            .result
            .ok_or_else(|| Error::Server("Missing result in response".to_string()))?;

        Ok(serde_json::from_value(result)?)
    }

    /// List available tools
    ///
    /// This method sends a request to the server to list all available tools
    /// that the client can use. Tools are identified by their names.
    ///
    /// # Returns
    ///
    /// Returns a `Result<ToolsListResponse>` containing the list of tools.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - The server returns an error response
    /// - The response is missing required fields
    /// - JSON deserialization of the response fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use mocopr_client::McpClient;
    /// # use mocopr_core::prelude::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let client = McpClient::connect_stdio("python", &["server.py"],
    ///     Implementation { name: "My Client".to_string(), version: "1.0.0".to_string() },
    ///     ClientCapabilities::default()).await?;
    ///
    /// // List tools
    /// let tools = client.list_tools().await?;
    /// for tool in tools.tools {
    ///     println!("Tool: {} - {}", tool.name, tool.description.unwrap_or_default());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_tools(&self) -> Result<ToolsListResponse> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Protocol::generate_request_id()),
            method: "tools/list".to_string(),
            params: Some(serde_json::to_value(&ToolsListRequest::new())?),
        };

        let response = self.session.send_request(request).await?;
        if let Some(error) = response.error {
            return Err(Error::Server(error.message));
        }

        let result = response
            .result
            .ok_or_else(|| Error::Server("Missing result in response".to_string()))?;

        Ok(serde_json::from_value(result)?)
    }

    /// Call a tool
    ///
    /// This method calls a specific tool by name with the given arguments.
    /// The tool is expected to perform some action and return the result.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the tool to call
    /// * `arguments` - Optional arguments to pass to the tool
    ///
    /// # Returns
    ///
    /// Returns a `Result<ToolsCallResponse>` containing the tool's result.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - The server returns an error response
    /// - The response is missing required fields
    /// - JSON deserialization of the response fails
    /// - The tool name is invalid or the tool is not available
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use mocopr_client::McpClient;
    /// # use mocopr_core::prelude::*;
    /// # use serde_json::json;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let client = McpClient::connect_stdio("python", &["server.py"],
    ///     Implementation { name: "My Client".to_string(), version: "1.0.0".to_string() },
    ///     ClientCapabilities::default()).await?;
    ///
    /// // Call a tool with no arguments
    /// let response = client.call_tool("list_files".to_string(), None).await?;
    /// println!("List of files: {:?}", response.content);
    ///
    /// // Call a tool with arguments
    /// let response = client.call_tool("calculate".to_string(), Some(json!({"expression": "2 + 2"}))).await?;
    /// println!("Calculation result: {:?}", response.content);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn call_tool(
        &self,
        name: String,
        arguments: Option<serde_json::Value>,
    ) -> Result<ToolsCallResponse> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Protocol::generate_request_id()),
            method: "tools/call".to_string(),
            params: Some(serde_json::to_value(&ToolsCallRequest { name, arguments })?),
        };

        let response = self.session.send_request(request).await?;
        if let Some(error) = response.error {
            return Err(Error::Server(error.message));
        }

        let result = response
            .result
            .ok_or_else(|| Error::Server("Missing result in response".to_string()))?;

        Ok(serde_json::from_value(result)?)
    }

    /// List available prompts
    ///
    /// This method sends a request to the server to list all available prompts
    /// that the client can use. Prompts are used to generate or modify content
    /// based on templates and variables.
    ///
    /// # Returns
    ///
    /// Returns a `Result<PromptsListResponse>` containing the list of prompts.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - The server returns an error response
    /// - The response is missing required fields
    /// - JSON deserialization of the response fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use mocopr_client::McpClient;
    /// # use mocopr_core::prelude::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let client = McpClient::connect_stdio("python", &["server.py"],
    ///     Implementation { name: "My Client".to_string(), version: "1.0.0".to_string() },
    ///     ClientCapabilities::default()).await?;
    ///
    /// // List prompts
    /// let prompts = client.list_prompts().await?;
    /// for prompt in prompts.prompts {
    ///     println!("Prompt: {} - {}", prompt.name, prompt.description.unwrap_or_default());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_prompts(&self) -> Result<PromptsListResponse> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Protocol::generate_request_id()),
            method: "prompts/list".to_string(),
            params: Some(serde_json::to_value(&PromptsListRequest::new())?),
        };

        let response = self.session.send_request(request).await?;
        if let Some(error) = response.error {
            return Err(Error::Server(error.message));
        }

        let result = response
            .result
            .ok_or_else(|| Error::Server("Missing result in response".to_string()))?;

        Ok(serde_json::from_value(result)?)
    }

    /// Get a prompt
    ///
    /// This method retrieves a specific prompt by name, optionally with
    /// arguments to customize the prompt. Prompts are used to generate or
    /// modify content based on templates and variables.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the prompt to retrieve
    /// * `arguments` - Optional arguments to customize the prompt
    ///
    /// # Returns
    ///
    /// Returns a `Result<PromptsGetResponse>` containing the prompt content.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - The server returns an error response
    /// - The response is missing required fields
    /// - JSON deserialization of the response fails
    /// - The prompt name is invalid or the prompt is not available
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use mocopr_client::McpClient;
    /// # use mocopr_core::prelude::*;
    /// # use serde_json::json;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let client = McpClient::connect_stdio("python", &["server.py"],
    ///     Implementation { name: "My Client".to_string(), version: "1.0.0".to_string() },
    ///     ClientCapabilities::default()).await?;
    ///
    /// // Get a prompt by name
    /// let response = client.get_prompt("greeting".to_string(), None).await?;
    /// println!("Prompt description: {:?}", response.description);
    ///
    /// // Get a prompt with arguments (note: arguments should be HashMap<String, String>)
    /// let mut args = std::collections::HashMap::new();
    /// args.insert("recipient".to_string(), "John".to_string());
    /// let response = client.get_prompt("email_template".to_string(), Some(args)).await?;
    /// println!("Prompt messages: {:?}", response.messages);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_prompt(
        &self,
        name: String,
        arguments: Option<std::collections::HashMap<String, String>>,
    ) -> Result<PromptsGetResponse> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Protocol::generate_request_id()),
            method: "prompts/get".to_string(),
            params: Some(serde_json::to_value(&PromptsGetRequest {
                name,
                arguments,
            })?),
        };

        let response = self.session.send_request(request).await?;
        if let Some(error) = response.error {
            return Err(Error::Server(error.message));
        }

        let result = response
            .result
            .ok_or_else(|| Error::Server("Missing result in response".to_string()))?;

        Ok(serde_json::from_value(result)?)
    }

    /// Send a ping to the server
    ///
    /// This method sends a ping request to the server to check if it's alive
    /// and responsive. It can optionally send a custom message with the ping.
    ///
    /// # Arguments
    ///
    /// * `message` - Optional custom message to send with the ping
    ///
    /// # Returns
    ///
    /// Returns a `Result<PingResponse>` containing the server's pong response.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - The server returns an error response
    /// - The response is missing required fields
    /// - JSON deserialization of the response fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use mocopr_client::McpClient;
    /// # use mocopr_core::prelude::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let client = McpClient::connect_stdio("python", &["server.py"],
    ///     Implementation { name: "My Client".to_string(), version: "1.0.0".to_string() },
    ///     ClientCapabilities::default()).await?;
    ///
    /// // Send a ping with a custom message
    /// let response = client.ping(Some("Hello, server!".to_string())).await?;
    /// println!("Ping response: {}", response.message.unwrap_or_default());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn ping(&self, message: Option<String>) -> Result<PingResponse> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Protocol::generate_request_id()),
            method: "ping".to_string(),
            params: Some(serde_json::to_value(&PingRequest { message })?),
        };

        let response = self.session.send_request(request).await?;
        if let Some(error) = response.error {
            return Err(Error::Server(error.message));
        }

        let result = response
            .result
            .ok_or_else(|| Error::Server("Missing result in response".to_string()))?;

        Ok(serde_json::from_value(result)?)
    }

    /// Close the client connection
    ///
    /// This method closes the underlying transport connection to the server,
    /// cleaning up any resources used by the client. After calling this method,
    /// the client can no longer be used to communicate with the server.
    ///
    /// # Returns
    ///
    /// Returns a `Result<()>` which is `Ok` if the connection was closed
    /// successfully.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - The transport layer encounters an error while closing
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use mocopr_client::McpClient;
    /// # use mocopr_core::prelude::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let client = McpClient::connect_stdio("python", &["server.py"],
    ///     Implementation { name: "My Client".to_string(), version: "1.0.0".to_string() },
    ///     ClientCapabilities::default()).await?;
    ///
    /// // Do some work with the client...
    ///
    /// // Close the client connection
    /// client.close().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn close(&self) -> Result<()> {
        self.session.close().await
    }

    /// Get session state
    ///
    /// This method retrieves the current state of the session, which can be
    /// used to determine if the client is connected, disconnected, or in the
    /// process of reconnecting.
    ///
    /// # Returns
    ///
    /// Returns the current `SessionState`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use mocopr_client::McpClient;
    /// # use mocopr_core::prelude::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let client = McpClient::connect_stdio("python", &["server.py"],
    ///     Implementation { name: "My Client".to_string(), version: "1.0.0".to_string() },
    ///     ClientCapabilities::default()).await?;
    ///
    /// // Get and print the session state
    /// let state = client.session_state().await;
    /// println!("Session state: {:?}", state);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn session_state(&self) -> SessionState {
        self.session.state().await
    }

    /// Check if the client is connected
    ///
    /// This method checks if the client is currently connected to the server.
    /// It returns `true` if connected, `false` otherwise.
    ///
    /// # Returns
    ///
    /// Returns `true` if the client is connected, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use mocopr_client::McpClient;
    /// # use mocopr_core::prelude::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let client = McpClient::connect_stdio("python", &["server.py"],
    ///     Implementation { name: "My Client".to_string(), version: "1.0.0".to_string() },
    ///     ClientCapabilities::default()).await?;
    ///
    /// // Check if the client is connected
    /// if client.is_connected().await {
    ///     println!("Client is connected");
    /// } else {
    ///     println!("Client is not connected");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn is_connected(&self) -> bool {
        self.session.is_connected().await
    }

    /// Get information about this client implementation.
    ///
    /// Returns the implementation details (name and version) that were provided
    /// when the client was created.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use mocopr_client::McpClient;
    /// # use mocopr_core::prelude::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// # let client = McpClient::connect_stdio("python", &["server.py"],
    /// #     Implementation { name: "My Client".to_string(), version: "1.0.0".to_string() },
    /// #     ClientCapabilities::default()).await?;
    /// let info = client.client_info();
    /// println!("Client: {} v{}", info.name, info.version);
    /// # Ok(())
    /// # }
    /// ```
    pub fn client_info(&self) -> &Implementation {
        &self.info
    }

    /// Get the capabilities this client supports.
    ///
    /// Returns the client capabilities that were provided when the client was
    /// created and negotiated with the server.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use mocopr_client::McpClient;
    /// # use mocopr_core::prelude::*;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// # let client = McpClient::connect_stdio("python", &["server.py"],
    /// #     Implementation { name: "My Client".to_string(), version: "1.0.0".to_string() },
    /// #     ClientCapabilities::default()).await?;
    /// let capabilities = client.client_capabilities();
    /// println!("Supports roots: {:?}", capabilities.roots);
    /// # Ok(())
    /// # }
    /// ```
    pub fn client_capabilities(&self) -> &ClientCapabilities {
        &self.capabilities
    }
}

/// Builder for creating MCP clients
pub struct McpClientBuilder {
    client_info: Option<Implementation>,
    capabilities: ClientCapabilities,
}

impl McpClientBuilder {
    /// Create a new `McpClientBuilder`.
    ///
    /// This function returns a new instance of `McpClientBuilder` with default
    /// settings. Use the builder methods to configure the client options.
    ///
    /// # Returns
    ///
    /// Returns a new `McpClientBuilder` instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_client::McpClientBuilder;
    ///
    /// // Create a new client builder
    /// let builder = McpClientBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            client_info: None,
            capabilities: ClientCapabilities::default(),
        }
    }

    /// Set the client information.
    ///
    /// This method sets the `Implementation` information for the client, which
    /// includes the client name and version. This information is sent to the
    /// server during the MCP handshake.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the client implementation
    /// * `version` - The version of the client implementation
    ///
    /// # Returns
    ///
    /// Returns the updated `McpClientBuilder` instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_client::McpClientBuilder;
    ///
    /// let builder = McpClientBuilder::new()
    ///     .with_info("My Client".to_string(), "1.0.0".to_string());
    /// ```
    pub fn with_info(mut self, name: String, version: String) -> Self {
        self.client_info = Some(Implementation { name, version });
        self
    }

    /// Enable sampling capability.
    ///
    /// This method enables the sampling capability for the client, which allows
    /// the client to request sampled data from the server. Sampling is useful
    /// for reducing the amount of data transferred over the network.
    ///
    /// # Returns
    ///
    /// Returns the updated `McpClientBuilder` instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_client::McpClientBuilder;
    ///
    /// let builder = McpClientBuilder::new().with_sampling();
    /// ```
    pub fn with_sampling(mut self) -> Self {
        self.capabilities = self.capabilities.with_sampling();
        self
    }

    /// Enable roots capability.
    ///
    /// This method enables the roots capability for the client, which allows
    /// the client to request the list of available roots from the server. Roots
    /// are the top-level elements in the MCP hierarchy.
    ///
    /// # Arguments
    ///
    /// * `list_changed` - If true, the server will only return changed roots
    ///   since the last request. Otherwise, all roots will be returned.
    ///
    /// # Returns
    ///
    /// Returns the updated `McpClientBuilder` instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_client::McpClientBuilder;
    ///
    /// let builder = McpClientBuilder::new().with_roots(true);
    /// ```
    pub fn with_roots(mut self, list_changed: bool) -> Self {
        self.capabilities = self.capabilities.with_roots(list_changed);
        self
    }

    /// Enable experimental features.
    ///
    /// This method enables experimental features for the client. Experimental
    /// features are not guaranteed to be stable and may change or be removed
    /// in future versions.
    ///
    /// # Arguments
    ///
    /// * `key` - The feature key
    /// * `value` - The feature value
    ///
    /// # Returns
    ///
    /// Returns the updated `McpClientBuilder` instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_client::McpClientBuilder;
    /// use serde_json::json;
    ///
    /// let builder = McpClientBuilder::new()
    ///     .with_experimental("new_feature".to_string(), json!({"enabled": true}));
    /// ```
    pub fn with_experimental(mut self, key: String, value: serde_json::Value) -> Self {
        self.capabilities = self.capabilities.with_experimental(key, value);
        self
    }

    /// Connect to an MCP server via stdio (process communication).
    ///
    /// This is a convenience method for connecting to MCP servers that run as
    /// separate processes and communicate via stdin/stdout. This is one of the
    /// most common ways to connect to MCP servers.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute (e.g., "python", "node", "./my-server")
    /// * `args` - Command line arguments to pass to the command
    ///
    /// # Returns
    ///
    /// Returns a `Result<McpClient>` which is `Ok` if the server process was
    /// started successfully and the MCP handshake completed.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - The command cannot be executed (not found, permissions, etc.)
    /// - The server process fails to start or crashes immediately
    /// - The server doesn't implement MCP protocol correctly
    /// - Capability negotiation fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use mocopr_client::McpClient;
    /// use mocopr_core::prelude::*;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// // Connect to a Python MCP server
    /// let client = McpClient::connect_stdio(
    ///     "python",
    ///     &["path/to/server.py"],
    ///     Implementation {
    ///         name: "My Python Client".to_string(),
    ///         version: "1.0.0".to_string(),
    ///     },
    ///     ClientCapabilities::default(),
    /// ).await?;
    ///
    /// // Connect to a Node.js MCP server
    /// let client = McpClient::connect_stdio(
    ///     "node",
    ///     &["server.js"],
    ///     Implementation {
    ///         name: "My Node Client".to_string(),
    ///         version: "1.0.0".to_string(),
    ///     },
    ///     ClientCapabilities::default(),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_stdio(self, command: &str, args: &[&str]) -> Result<McpClient> {
        let client_info = self
            .client_info
            .ok_or_else(|| Error::InvalidRequest("Client info is required".to_string()))?;

        McpClient::connect_stdio(command, args, client_info, self.capabilities).await
    }

    /// Connect to an MCP server via WebSocket
    ///
    /// This is a convenience method for connecting to MCP servers over the network
    /// using the WebSocket protocol. This is useful for real-time communication
    /// with MCP servers.
    ///
    /// # Arguments
    ///
    /// * `url` - The WebSocket URL of the MCP server (e.g., "ws://localhost:8080")
    ///
    /// # Returns
    ///
    /// Returns a `Result<McpClient>` which is `Ok` if the WebSocket connection
    /// was established successfully and the MCP handshake completed.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - The WebSocket connection fails (invalid URL, server not reachable, etc.)
    /// - The server doesn't implement MCP protocol correctly
    /// - Capability negotiation fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use mocopr_client::McpClient;
    /// use mocopr_core::prelude::*;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let client = McpClient::connect_websocket(
    ///     "ws://localhost:8080",
    ///     Implementation {
    ///         name: "My Client".to_string(),
    ///         version: "1.0.0".to_string(),
    ///     },
    ///     ClientCapabilities::default(),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_websocket(self, url: &str) -> Result<McpClient> {
        let client_info = self
            .client_info
            .ok_or_else(|| Error::InvalidRequest("Client info is required".to_string()))?;

        McpClient::connect_websocket(url, client_info, self.capabilities).await
    }
}

impl Default for McpClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Prelude for MCP client development
pub mod prelude {
    pub use crate::*;
    pub use mocopr_core::prelude::*;
}
