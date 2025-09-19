//! High-level MCP server implementation

use crate::middleware::Middleware;
use crate::registry::*;
use axum::extract::ws::WebSocket;
use mocopr_core::monitoring::MonitoringSystem;
use bytes::{BufMut, BytesMut};
use mocopr_core::prelude::*;
use mocopr_core::utils::json;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

/// High-level MCP server
pub struct McpServer {
    info: Implementation,
    capabilities: ServerCapabilities,
    handler: Arc<ServerMessageHandler>,
    middleware_stack: Vec<Box<dyn Middleware>>,
    monitoring_system: Option<MonitoringSystem>,
    bind_address: String,
    port: u16,
    enable_http: bool,
    enable_websocket: bool,
    multi_threaded_runtime: bool,
    shutdown_tx: watch::Sender<()>,
    shutdown_rx: watch::Receiver<()>,
}

impl McpServer {
    /// Create a new server builder
    pub fn builder() -> crate::builder::McpServerBuilder {
        crate::builder::McpServerBuilder::new()
    }

    /// Create a new MCP server
    pub fn new(
        info: Implementation,
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
        multi_threaded_runtime: bool,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let handler = Arc::new(ServerMessageHandler::new(
            info.clone(),
            capabilities.clone(),
            resource_registry,
            tool_registry,
            prompt_registry,
        ));

        Self {
            info,
            capabilities,
            handler,
            middleware_stack,
            monitoring_system,
            bind_address,
            port,
            enable_http,
            enable_websocket,
            multi_threaded_runtime,
            shutdown_tx,
            shutdown_rx,
        }
    }

    /// Get server info
    pub fn info(&self) -> &Implementation {
        &self.info
    }

    /// Get server capabilities
    pub fn capabilities(&self) -> &ServerCapabilities {
        &self.capabilities
    }

    /// Get the middleware stack
    pub fn middleware(&self) -> &Vec<Box<dyn Middleware>> {
        &self.middleware_stack
    }

    /// Get the monitoring system (if enabled)
    pub fn monitoring(&self) -> Option<&MonitoringSystem> {
        self.monitoring_system.as_ref()
    }

    /// Get the configured bind address
    pub fn bind_address(&self) -> &str {
        &self.bind_address
    }

    /// Get the configured port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Check if HTTP transport is enabled
    pub fn http_enabled(&self) -> bool {
        self.enable_http
    }

    /// Check if WebSocket transport is enabled
    pub fn websocket_enabled(&self) -> bool {
        self.enable_websocket
    }

    /// Trigger a graceful shutdown of the server.
    pub fn shutdown(&self) -> Result<()> {
        self.shutdown_tx.send(()).map_err(|e| Error::Internal(e.to_string()))
    }

    /// Run the server using stdio transport
    pub async fn run_stdio(&self) -> Result<()> {
        info!("Starting MCP server with stdio transport");

        let transport = mocopr_core::transport::stdio::StdioTransport::current_process();
        let (session, mut events) =
            mocopr_core::protocol::Session::new(Box::new(transport), self.handler.clone());

        let session = Arc::new(session);
        let session_clone = session.clone();

        // Handle session events in the background
        let session_events = tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                match event {
                    mocopr_core::protocol::SessionEvent::Connected => {
                        info!("Client connected");
                    }
                    mocopr_core::protocol::SessionEvent::Disconnected => {
                        info!("Client disconnected");
                        break;
                    }
                    mocopr_core::protocol::SessionEvent::Initialized { client_info } => {
                        info!("Session initialized with client: {}", client_info.name);
                    }
                    mocopr_core::protocol::SessionEvent::Error { error } => {
                        error!("Session error: {}", error);
                    }
                    _ => {}
                }
            }
        });

        // Run the session
        let mut shutdown_rx = self.shutdown_rx.clone();
        let session_run = tokio::spawn(async move {
            session_clone.run().await
        });

        tokio::select! {
            res = session_run => {
                if let Ok(Err(e)) = res {
                    error!("Session exited with error: {}", e);
                }
            },
            _ = shutdown_rx.changed() => {
                info!("Graceful shutdown initiated for stdio transport");
                if let Err(e) = session.shutdown().await {
                    error!("Failed to shutdown session: {}", e);
                }
            }
        };

        let _ = session_events.await;
        Ok(())
    }

    /// Run the server with configured transports
    ///
    /// This will start the server using HTTP and/or WebSocket transports
    /// if they were enabled during building, falling back to stdio if neither is enabled.
    pub async fn run(&self) -> Result<()> {
        if self.multi_threaded_runtime {
            warn!("Multi-threaded runtime requested, but the `run` method does not create a new runtime. Please use the `#[tokio::main(flavor = \"multi_thread\")]` attribute on your main function to enable the multi-threaded runtime.");
        }

        if self.enable_http && self.enable_websocket {
            // Both HTTP and WebSocket enabled - start both
            let addr = format!("{}:{}", self.bind_address, self.port);
            info!("Starting server with HTTP and WebSocket on {}", addr);
            self.run_http_with_websocket(&addr).await
        } else if self.enable_websocket {
            // Just WebSocket
            let addr = format!("{}:{}", self.bind_address, self.port);
            self.run_websocket(&addr).await
        } else if self.enable_http {
            // Just HTTP
            let addr = format!("{}:{}", self.bind_address, self.port);
            self.run_http(&addr).await
        } else {
            // Default to stdio
            self.run_stdio().await
        }
    }

    /// Run the server using HTTP transport
    pub async fn run_http(&self, addr: &str) -> Result<()> {
        info!("Starting MCP server with HTTP transport on {}", addr);

        use axum::{Router, routing::post};
        use tower_http::cors::CorsLayer;

        let handler = self.handler.clone();

        let app = Router::new()
            .route("/mcp", post(handle_http_request))
            .layer(CorsLayer::permissive())
            .with_state(handler);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!("HTTP server listening on {}", addr);

        let mut shutdown_rx = self.shutdown_rx.clone();
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                shutdown_rx.changed().await.ok();
            })
            .await?;
        Ok(())
    }

    /// Run the server with both HTTP and WebSocket support
    pub async fn run_http_with_websocket(&self, addr: &str) -> Result<()> {
        info!(
            "Starting MCP server with HTTP and WebSocket transport on {}",
            addr
        );

        use axum::{
            Router,
            extract::ws::WebSocketUpgrade,
            routing::{get, post},
        };
        use tower_http::cors::CorsLayer;

        let handler = self.handler.clone();
        let ws_handler = handler.clone();

        let app = Router::new()
            .route("/mcp", post(handle_http_request))
            .route(
                "/mcp/ws",
                get(move |ws: WebSocketUpgrade| async move {
                    ws.on_upgrade(move |socket| handle_websocket(socket, ws_handler))
                }),
            )
            .layer(CorsLayer::permissive())
            .with_state(handler);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!("HTTP+WebSocket server listening on {}", addr);

        let mut shutdown_rx = self.shutdown_rx.clone();
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                shutdown_rx.changed().await.ok();
            })
            .await?;
        Ok(())
    }

    /// Run the server using WebSocket transport
    pub async fn run_websocket(&self, addr: &str) -> Result<()> {
        info!("Starting MCP server with WebSocket transport on {}", addr);

        use axum::{Router, extract::ws::WebSocketUpgrade, routing::get};
        use tower_http::cors::CorsLayer;

        let handler = self.handler.clone();

        let app = Router::new()
            .route(
                "/mcp",
                get(move |ws: WebSocketUpgrade| async move {
                    ws.on_upgrade(move |socket| handle_websocket(socket, handler))
                }),
            )
            .layer(CorsLayer::permissive());

        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!("WebSocket server listening on {}", addr);

        let mut shutdown_rx = self.shutdown_rx.clone();
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                shutdown_rx.changed().await.ok();
            })
            .await?;
        Ok(())
    }

    /// Get a reference to the resource registry
    pub fn resources(&self) -> &ResourceRegistry {
        &self.handler.resources
    }

    /// Get a reference to the tool registry
    pub fn tools(&self) -> &ToolRegistry {
        &self.handler.tools
    }

    /// Get a reference to the prompt registry
    pub fn prompts(&self) -> &PromptRegistry {
        &self.handler.prompts
    }
}

/// Route MCP method calls to appropriate handlers
async fn handle_mcp_method(
    handler: &Arc<ServerMessageHandler>,
    json_msg: &serde_json::Value,
) -> Option<JsonRpcMessage> {
    let id: Option<RequestId> = json_msg
        .get("id")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let method = match json_msg.get("method").and_then(|m| m.as_str()) {
        Some(method) => method,
        None => {
            return Some(JsonRpcMessage::error(
                id,
                -32600,
                "Invalid Request: Missing 'method' field.",
            ));
        }
    };

    let params = json_msg.get("params").cloned().unwrap_or(serde_json::Value::Null);

    // Helper macro to parse request parameters
    macro_rules! parse_request {
        ($type:ty) => {
            match serde_json::from_value::<$type>(params.clone()) {
                Ok(req) => req,
                Err(e) => {
                    return Some(JsonRpcMessage::error(
                        id,
                        -32602,
                        format!("Invalid Parameters: {}", e),
                    ));
                }
            }
        };
        ($type:ty, default) => {
            match serde_json::from_value::<$type>(params.clone()) {
                Ok(req) => req,
                Err(_) => <$type>::default(),
            }
        };
    }

    let result = match method {
        "ping" => {
            let request = parse_request!(PingRequest, default);
            handler
                .handle_ping(request)
                .await
                .and_then(|r| serde_json::to_value(r).map_err(Error::from))
        }
        "resources/list" => {
            let request = parse_request!(ResourcesListRequest, default);
            handler
                .handle_resources_list(request)
                .await
                .and_then(|r| serde_json::to_value(r).map_err(Error::from))
        }
        "resources/read" => {
            let request = parse_request!(ResourcesReadRequest);
            handler
                .handle_resources_read(request)
                .await
                .and_then(|r| serde_json::to_value(r).map_err(Error::from))
        }
        "resources/subscribe" => {
            let request = parse_request!(ResourcesSubscribeRequest);
            handler
                .handle_resources_subscribe(request)
                .await
                .and_then(|r| serde_json::to_value(r).map_err(Error::from))
        }
        "resources/unsubscribe" => {
            let request = parse_request!(ResourcesUnsubscribeRequest);
            handler
                .handle_resources_unsubscribe(request)
                .await
                .and_then(|r| serde_json::to_value(r).map_err(Error::from))
        }
        "tools/list" => {
            let request = parse_request!(ToolsListRequest, default);
            handler
                .handle_tools_list(request)
                .await
                .and_then(|r| serde_json::to_value(r).map_err(Error::from))
        }
        "tools/call" => {
            let request = parse_request!(ToolsCallRequest);
            handler
                .handle_tools_call(request)
                .await
                .and_then(|r| serde_json::to_value(r).map_err(Error::from))
        }
        "prompts/list" => {
            let request = parse_request!(PromptsListRequest, default);
            handler
                .handle_prompts_list(request)
                .await
                .and_then(|r| serde_json::to_value(r).map_err(Error::from))
        }
        "prompts/get" => {
            let request = parse_request!(PromptsGetRequest);
            handler
                .handle_prompts_get(request)
                .await
                .and_then(|r| serde_json::to_value(r).map_err(Error::from))
        }
        "logging/setLevel" => {
            let request = parse_request!(LoggingSetLevelRequest);
            handler
                .handle_logging_set_level(request)
                .await
                .and_then(|r| serde_json::to_value(r).map_err(Error::from))
        }
        "notifications/initialized" => {
            let notification = parse_request!(InitializedNotification, default);
            if let Err(e) = handler.handle_initialized(notification).await {
                warn!("Error handling initialized notification: {}", e);
            }
            return None;
        }
        _ => Err(Error::MethodNotFound(method.to_string())),
    };

    // Convert result to JSON response
    match result {
        Ok(value) => Some(JsonRpcMessage::success(id, value)),
        Err(e) => Some(JsonRpcMessage::from_error(id, e)),
    }
}

/// Handle WebSocket connections
async fn handle_websocket(mut socket: WebSocket, handler: Arc<ServerMessageHandler>) {
    info!("WebSocket client connected");
    let mut initialized = false;

    while let Some(result) = socket.recv().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
        };

        let text = if let Ok(text) = msg.to_text() {
            text
        } else {
            warn!("Received non-text WebSocket message, ignoring");
            continue;
        };

        debug!("Received WebSocket message: {}", text);

        let json_msg: serde_json::Value = match json::from_str(text) {
            Ok(val) => val,
            Err(e) => {
                error!("Failed to parse JSON message: {}", e);
                let error_response = JsonRpcMessage::error(None, -32700, "Parse error");
                if socket.send(axum::extract::ws::Message::Text(serde_json::to_string(&error_response).unwrap())).await.is_err() {
                    error!("Failed to send parse error response");
                }
                continue;
            }
        };

        let id: Option<RequestId> = json_msg.get("id").and_then(|v| serde_json::from_value(v.clone()).ok());

        if !initialized {
            if let Some("initialize") = json_msg.get("method").and_then(|m| m.as_str()) {
                match serde_json::from_value::<InitializeRequest>(json_msg) {
                    Ok(init_request) => match handler.handle_initialize(init_request).await {
                        Ok(init_response) => {
                            initialized = true;
                            let response = JsonRpcMessage::success(id, init_response);
                            if socket.send(axum::extract::ws::Message::Text(serde_json::to_string(&response).unwrap())).await.is_err() {
                                error!("Failed to send initialize response");
                                break;
                            }
                        }
                        Err(e) => {
                            let response = JsonRpcMessage::from_error(id, e);
                            if socket.send(axum::extract::ws::Message::Text(serde_json::to_string(&response).unwrap())).await.is_err() {
                                error!("Failed to send initialize error response");
                                break;
                            }
                        }
                    },
                    Err(e) => {
                        let response = JsonRpcMessage::error(id, -32602, format!("Invalid initialize request: {}", e));
                        if socket.send(axum::extract::ws::Message::Text(serde_json::to_string(&response).unwrap())).await.is_err() {
                            error!("Failed to send invalid initialize request response");
                            break;
                        }
                    }
                }
            } else {
                let response = JsonRpcMessage::error(id, -32002, "Server not initialized");
                if socket.send(axum::extract::ws::Message::Text(serde_json::to_string(&response).unwrap())).await.is_err() {
                    error!("Failed to send 'not initialized' error response");
                    break;
                }
            }
        } else if let Some(response) = handle_mcp_method(&handler, &json_msg).await {
            if socket.send(axum::extract::ws::Message::Text(serde_json::to_string(&response).unwrap())).await.is_err() {
                error!("Failed to send WebSocket response");
                break;
            }
        }
    }

    info!("WebSocket client disconnected");
}

/// Server message handler that implements the MCP protocol
pub struct ServerMessageHandler {
    pub info: Implementation,
    pub capabilities: ServerCapabilities,
    pub resources: ResourceRegistry,
    pub tools: ToolRegistry,
    pub prompts: PromptRegistry,
}

impl ServerMessageHandler {
    pub fn new(
        info: Implementation,
        capabilities: ServerCapabilities,
        resources: ResourceRegistry,
        tools: ToolRegistry,
        prompts: PromptRegistry,
    ) -> Self {
        Self {
            info,
            capabilities,
            resources,
            tools,
            prompts,
        }
    }
}

#[async_trait::async_trait]
impl MessageHandler for ServerMessageHandler {
    async fn handle_initialize(&self, request: InitializeRequest) -> Result<InitializeResponse> {
        // Validate protocol version
        if !Protocol::is_version_supported(&request.protocol_version) {
            return Err(Error::InvalidRequest(format!(
                "Unsupported protocol version: {}",
                request.protocol_version
            )));
        }

        info!(
            "Client initialized: {} v{}",
            request.client_info.name, request.client_info.version
        );

        Ok(InitializeResponse {
            protocol_version: Protocol::latest_version().to_string(),
            capabilities: self.capabilities.clone(),
            server_info: self.info.clone(),
            instructions: None,
        })
    }

    async fn handle_resources_list(
        &self,
        request: ResourcesListRequest,
    ) -> Result<ResourcesListResponse> {
        self.resources.list_resources(request).await
    }

    async fn handle_resources_read(
        &self,
        request: ResourcesReadRequest,
    ) -> Result<ResourcesReadResponse> {
        self.resources.read_resource(request).await
    }

    async fn handle_resources_subscribe(
        &self,
        request: ResourcesSubscribeRequest,
    ) -> Result<ResourcesSubscribeResponse> {
        self.resources.subscribe_resource(request).await
    }

    async fn handle_resources_unsubscribe(
        &self,
        request: ResourcesUnsubscribeRequest,
    ) -> Result<ResourcesUnsubscribeResponse> {
        self.resources.unsubscribe_resource(request).await
    }

    async fn handle_tools_list(&self, request: ToolsListRequest) -> Result<ToolsListResponse> {
        self.tools.list_tools(request).await
    }

    async fn handle_tools_call(&self, request: ToolsCallRequest) -> Result<ToolsCallResponse> {
        self.tools.call_tool(request).await
    }

    async fn handle_prompts_list(
        &self,
        request: PromptsListRequest,
    ) -> Result<PromptsListResponse> {
        self.prompts.list_prompts(request).await
    }

    async fn handle_prompts_get(&self, request: PromptsGetRequest) -> Result<PromptsGetResponse> {
        self.prompts.get_prompt(request).await
    }
}

/// HTTP request handler for MCP over HTTP
async fn handle_http_request(
    axum::extract::State(handler): axum::extract::State<Arc<ServerMessageHandler>>,
    axum::Json(request): axum::Json<serde_json::Value>,
) -> axum::Json<serde_json::Value> {
    if let Some(response) = handle_mcp_method(&handler, &request).await {
        axum::Json(serde_json::to_value(response).unwrap())
    } else {
        // This case is for notifications, which don't have a response.
        // HTTP doesn't really have a concept of notifications in the same way as WebSocket,
        // so we'll return an empty response with a 204 No Content status code.
        // However, Axum's Json type doesn't directly support changing the status code,
        // so for now we'll return an empty JSON object.
        axum::Json(json!({}))
    }
}
