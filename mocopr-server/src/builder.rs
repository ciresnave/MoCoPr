//! Builder pattern for creating MCP servers.
//!
//! This module provides a fluent builder API for configuring and creating MCP servers.
//! The builder pattern allows for easy configuration of server capabilities, handlers,
//! and other settings before starting the server.
//!
//! # Examples
//!
//! ```rust
//! use mocopr_server::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let server = McpServerBuilder::new()
//!         .with_info("My Server", "1.0.0")
//!         .with_resources()
//!         .with_tools()
//!         .with_prompts()
//!         .build()?;
//!
//!     server.run_stdio().await?;
//!     Ok(())
//! }
//! ```

use crate::handlers::*;
use crate::middleware::Middleware;
use crate::registry::*;
use crate::server::McpServer;
use mocopr_core::monitoring::MonitoringSystem;
use mocopr_core::prelude::*;

/// Builder for creating MCP servers with a fluent API.
///
/// The builder pattern allows you to configure all aspects of an MCP server
/// before building and starting it. This includes setting server metadata,
/// enabling capabilities, and registering handlers for resources, tools, and prompts.
///
/// # Examples
///
/// ```rust
/// use mocopr_server::prelude::*;
///
/// let builder = McpServerBuilder::new()
///     .with_info("File Server", "1.0.0")
///     .with_resources()
///     .with_tools();
/// ```
pub struct McpServerBuilder {
    name: Option<String>,
    version: Option<String>,
    capabilities: ServerCapabilities,
    resource_registry: ResourceRegistry,
    tool_registry: ToolRegistry,
    prompt_registry: PromptRegistry,
    middleware_stack: Vec<Box<dyn Middleware>>,
    monitoring_system: Option<MonitoringSystem>,
    bind_address: String,
    port: u16,
    enable_http: bool,
    enable_websocket: bool,
}

impl McpServerBuilder {
    /// Create a new server builder.
    ///
    /// Returns a builder with default settings that can be configured
    /// using the fluent API methods.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_server::McpServerBuilder;
    ///
    /// let builder = McpServerBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            name: None,
            version: None,
            capabilities: ServerCapabilities::default(),
            resource_registry: ResourceRegistry::new(),
            tool_registry: ToolRegistry::new(),
            prompt_registry: PromptRegistry::new(),
            middleware_stack: Vec::new(),
            monitoring_system: None,
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            enable_http: false,
            enable_websocket: false,
        }
    }

    /// Set server name and version.
    ///
    /// This information is sent to clients during the initialization handshake
    /// and helps identify your server implementation.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of your MCP server
    /// * `version` - The version of your server implementation
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_server::McpServerBuilder;
    ///
    /// let builder = McpServerBuilder::new()
    ///     .with_info("My File Server", "1.2.3");
    /// ```
    pub fn with_info(mut self, name: impl Into<String>, version: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self.version = Some(version.into());
        self
    }

    /// Enable logging capability.
    ///
    /// This allows the server to receive logging configuration requests
    /// from clients and send log messages.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_server::McpServerBuilder;
    ///
    /// let builder = McpServerBuilder::new()
    ///     .with_logging();
    /// ```
    pub fn with_logging(mut self) -> Self {
        self.capabilities = self.capabilities.with_logging();
        self
    }

    /// Enable resources capability with default settings.
    ///
    /// This enables the server to provide resources with both list change
    /// notifications and subscription support enabled.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_server::McpServerBuilder;
    ///
    /// let builder = McpServerBuilder::new()
    ///     .with_resources();
    /// ```
    pub fn with_resources(mut self) -> Self {
        self.capabilities = self.capabilities.with_resources(true, true);
        self
    }

    /// Enable resources capability with specific settings.
    ///
    /// # Arguments
    ///
    /// * `list_changed` - Whether to support resource list change notifications
    /// * `subscribe` - Whether to support resource content subscriptions
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_server::McpServerBuilder;
    ///
    /// let builder = McpServerBuilder::new()
    ///     .with_resources_config(true, false); // List changes but no subscriptions
    /// ```
    pub fn with_resources_config(mut self, list_changed: bool, subscribe: bool) -> Self {
        self.capabilities = self.capabilities.with_resources(list_changed, subscribe);
        self
    }

    /// Enable tools capability
    pub fn with_tools(mut self) -> Self {
        self.capabilities = self.capabilities.with_tools(true);
        self
    }

    /// Enable tools capability with specific settings
    pub fn with_tools_config(mut self, list_changed: bool) -> Self {
        self.capabilities = self.capabilities.with_tools(list_changed);
        self
    }

    /// Enable prompts capability
    pub fn with_prompts(mut self) -> Self {
        self.capabilities = self.capabilities.with_prompts(true);
        self
    }

    /// Enable prompts capability with specific settings
    pub fn with_prompts_config(mut self, list_changed: bool) -> Self {
        self.capabilities = self.capabilities.with_prompts(list_changed);
        self
    }

    /// Add a resource handler
    pub fn with_resource<R>(mut self, resource: R) -> Self
    where
        R: ResourceHandler + 'static,
    {
        self.resource_registry.register(Box::new(resource));
        self
    }

    /// Add a tool handler
    pub fn with_tool<T>(mut self, tool: T) -> Self
    where
        T: ToolHandler + 'static,
    {
        self.tool_registry.register(Box::new(tool));
        self
    }

    /// Add a prompt handler
    pub fn with_prompt<P>(mut self, prompt: P) -> Self
    where
        P: PromptHandler + 'static,
    {
        self.prompt_registry.register(Box::new(prompt));
        self
    }

    /// Add multiple resources
    pub fn with_resources_from<I, R>(mut self, resources: I) -> Self
    where
        I: IntoIterator<Item = R>,
        R: ResourceHandler + 'static,
    {
        for resource in resources {
            self.resource_registry.register(Box::new(resource));
        }
        self
    }

    /// Add multiple tools
    pub fn with_tools_from<I, T>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: ToolHandler + 'static,
    {
        for tool in tools {
            self.tool_registry.register(Box::new(tool));
        }
        self
    }

    /// Add multiple prompts
    pub fn with_prompts_from<I, P>(mut self, prompts: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: PromptHandler + 'static,
    {
        for prompt in prompts {
            self.prompt_registry.register(Box::new(prompt));
        }
        self
    }

    /// Add experimental capability
    pub fn with_experimental(mut self, key: String, value: serde_json::Value) -> Self {
        self.capabilities = self.capabilities.with_experimental(key, value);
        self
    }

    /// Add middleware to the server
    ///
    /// Middleware will be executed in the order it was added.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_server::prelude::*;
    /// use mocopr_server::middleware::LoggingMiddleware;
    ///
    /// let builder = McpServerBuilder::new()
    ///     .with_middleware(LoggingMiddleware::new());
    /// ```
    pub fn with_middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware + 'static,
    {
        self.middleware_stack.push(Box::new(middleware));
        self
    }

    /// Enable monitoring system
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_server::prelude::*;
    ///
    /// let builder = McpServerBuilder::new()
    ///     .with_monitoring();
    /// ```
    pub fn with_monitoring(mut self) -> Self {
        use mocopr_core::monitoring::MonitoringConfig;
        self.monitoring_system = Some(MonitoringSystem::new(MonitoringConfig::default()));
        self
    }

    /// Configure server address and port
    ///
    /// # Arguments
    ///
    /// * `address` - The IP address to bind to (e.g., "0.0.0.0", "127.0.0.1")
    /// * `port` - The port number to bind to
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_server::prelude::*;
    ///
    /// let builder = McpServerBuilder::new()
    ///     .with_bind_address("0.0.0.0", 8080);
    /// ```
    pub fn with_bind_address(mut self, address: impl Into<String>, port: u16) -> Self {
        self.bind_address = address.into();
        self.port = port;
        self
    }

    /// Enable HTTP transport
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_server::prelude::*;
    ///
    /// let builder = McpServerBuilder::new()
    ///     .with_http_transport();
    /// ```
    pub fn with_http_transport(mut self) -> Self {
        self.enable_http = true;
        self
    }

    /// Enable WebSocket transport
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_server::prelude::*;
    ///
    /// let builder = McpServerBuilder::new()
    ///     .with_websocket_transport();
    /// ```
    pub fn with_websocket_transport(mut self) -> Self {
        self.enable_websocket = true;
        self
    }

    /// Build the MCP server
    pub fn build(self) -> Result<McpServer> {
        let name = self
            .name
            .ok_or_else(|| Error::InvalidRequest("Server name is required".to_string()))?;

        let version = self
            .version
            .ok_or_else(|| Error::InvalidRequest("Server version is required".to_string()))?;

        let info = Implementation { name, version };

        Ok(McpServer::new(
            info,
            self.capabilities,
            self.resource_registry,
            self.tool_registry,
            self.prompt_registry,
            self.middleware_stack,
            self.monitoring_system,
            self.bind_address,
            self.port,
            self.enable_http,
            self.enable_websocket,
        ))
    }
}

impl Default for McpServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro for easily creating MCP servers
#[macro_export]
macro_rules! mcp_server {
    (
        name: $name:expr,
        version: $version:expr,
        $( $config:tt )*
    ) => {
        {
            let mut builder = $crate::McpServerBuilder::new()
                .with_info($name, $version);

            mcp_server_config!(builder, $( $config )*);
            builder
        }
    };
}

/// Helper macro for server configuration
#[macro_export]
macro_rules! mcp_server_config {
    ($builder:ident, resources: true, $( $rest:tt )*) => {
        $builder = $builder.with_resources();
        mcp_server_config!($builder, $( $rest )*);
    };
    ($builder:ident, tools: true, $( $rest:tt )*) => {
        $builder = $builder.with_tools();
        mcp_server_config!($builder, $( $rest )*);
    };
    ($builder:ident, prompts: true, $( $rest:tt )*) => {
        $builder = $builder.with_prompts();
        mcp_server_config!($builder, $( $rest )*);
    };
    ($builder:ident, logging: true, $( $rest:tt )*) => {
        $builder = $builder.with_logging();
        mcp_server_config!($builder, $( $rest )*);
    };
    ($builder:ident,) => {};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let server = McpServerBuilder::new()
            .with_info("Test Server", "1.0.0")
            .with_resources()
            .with_tools()
            .build()
            .unwrap();

        assert_eq!(server.info().name, "Test Server");
        assert_eq!(server.info().version, "1.0.0");
        assert!(server.capabilities().resources.is_some());
        assert!(server.capabilities().tools.is_some());
    }

    #[test]
    fn test_builder_validation() {
        let result = McpServerBuilder::new().with_resources().build();

        assert!(result.is_err());
    }
}
