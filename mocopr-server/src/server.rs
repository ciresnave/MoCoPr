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
        let result = tokio::select! {
            res = session.run() => {
                res
            },
            _ = self.shutdown_rx.clone().changed() => {
                info!("Graceful shutdown initiated for stdio transport");
                Ok(())
            }
        };
        let _ = session_events.await;
        result
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

        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                self.shutdown_rx.clone().changed().await.ok();
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

        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                self.shutdown_rx.clone().changed().await.ok();
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

        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                self.shutdown_rx.clone().changed().await.ok();
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
) -> Result<Option<JsonRpcMessage>> {
    let method = match json_msg.get("method").and_then(|m| m.as_str()) {
        Some(method) => method,
        None => {
            return Ok(Some(JsonRpcMessage::Response(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: serde_json::from_value(json_msg.get("id").cloned().unwrap_or(serde_json::Value::Null))
                    .unwrap_or(RequestId::Null),
                result: None,
                error: Some(JsonRpcError {
                    code: -32600,
                    message: "Invalid Request".to_string(),
                    data: None,
                }),
            })));
        }
    };

    let id = json_msg.get("id");
    let params = json_msg.get("params");

    // Handle different MCP methods
    let result = match method {
        "ping" => {
            let request = match params {
                Some(p) => serde_json::from_value::<PingRequest>(p.clone()).unwrap_or_default(),
                None => PingRequest::default(),
            };
            handler
                .handle_ping(request)
                .await
                .map(|r| serde_json::to_value(r).unwrap())
        }

        "resources/list" => {
            let request = match params {
                Some(p) => serde_json::from_value::<ResourcesListRequest>(p.clone())?,
                None => ResourcesListRequest::default(),
            };
            handler
                .handle_resources_list(request)
                .await
                .map(|r| serde_json::to_value(r).unwrap())
        }

        "resources/read" => {
            let request = match params {
                Some(p) => serde_json::from_value::<ResourcesReadRequest>(p.clone())?,
                None => return Err(mocopr_core::Error::InvalidParams("Missing params".to_string())),
            };
            handler
                .handle_resources_read(request)
                .await
                .map(|r| serde_json::to_value(r).unwrap())
        }

        "resources/subscribe" => {
            let request = match params {
                Some(p) => serde_json::from_value::<ResourcesSubscribeRequest>(p.clone())?,
                None => return Err(mocopr_core::Error::InvalidParams("Missing params".to_string())),
            };
            handler
                .handle_resources_subscribe(request)
                .await
                .map(|r| serde_json::to_value(r).unwrap())
        }

        "resources/unsubscribe" => {
            let request = match params {
                Some(p) => serde_json::from_value::<ResourcesUnsubscribeRequest>(p.clone())?,
                None => return Err(mocopr_core::Error::InvalidParams("Missing params".to_string())),
            };
            handler
                .handle_resources_unsubscribe(request)
                .await
                .map(|r| serde_json::to_value(r).unwrap())
        }

        "tools/list" => {
            let request = match params {
                Some(p) => serde_json::from_value::<ToolsListRequest>(p.clone())?,
                None => ToolsListRequest::default(),
            };
            handler
                .handle_tools_list(request)
                .await
                .map(|r| serde_json::to_value(r).unwrap())
        }

        "tools/call" => {
            let request = match params {
                Some(p) => serde_json::from_value::<ToolsCallRequest>(p.clone())?,
                None => return Err(mocopr_core::Error::InvalidParams("Missing params".to_string())),
            };
            handler
                .handle_tools_call(request)
                .await
                .map(|r| serde_json::to_value(r).unwrap())
        }

        "prompts/list" => {
            let request = match params {
                Some(p) => serde_json::from_value::<PromptsListRequest>(p.clone())?,
                None => PromptsListRequest::default(),
            };
            handler
                .handle_prompts_list(request)
                .await
                .map(|r| serde_json::to_value(r).unwrap())
        }

        "prompts/get" => {
            let request = match params {
                Some(p) => serde_json::from_value::<PromptsGetRequest>(p.clone())?,
                None => return Err(mocopr_core::Error::InvalidParams("Missing params".to_string())),
            };
            handler
                .handle_prompts_get(request)
                .await
                .map(|r| serde_json::to_value(r).unwrap())
        }

        "logging/setLevel" => {
            let request = match params {
                Some(p) => serde_json::from_value::<LoggingSetLevelRequest>(p.clone())?,
                None => return Err(mocopr_core::Error::InvalidParams("Missing params".to_string())),
            };
            handler
                .handle_logging_set_level(request)
                .await
                .map(|r| serde_json::to_value(r).unwrap())
        }

        // Handle notifications (no response expected)
        "notifications/initialized" => {
            let notification = match params {
                Some(p) => serde_json::from_value::<InitializedNotification>(p.clone())?,
                None => InitializedNotification::default(),
            };
            handler.handle_initialized(notification).await?;
            return Ok(None);
        }

        // Unknown method
        _ => Err(mocopr_core::Error::MethodNotFound(method.to_string())),
    };

    // Convert result to JSON response
    let response = match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: serde_json::from_value(id.cloned().unwrap_or(serde_json::Value::Null))
                .unwrap_or(RequestId::Null),
            result: Some(value),
            error: None,
        },
        Err(e) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: serde_json::from_value(id.cloned().unwrap_or(serde_json::Value::Null))
                .unwrap_or(RequestId::Null),
            result: None,
            error: Some(JsonRpcError {
                code: match &e {
                    mocopr_core::Error::MethodNotFound(_) => -32601,
                    mocopr_core::Error::InvalidRequest(_) => -32602,
                    _ => -32603,
                },
                message: e.to_string(),
                data: None,
            }),
        },
    };
    Ok(Some(JsonRpcMessage::Response(response)))
}

/// Handle WebSocket connections
async fn handle_websocket(mut socket: WebSocket, handler: Arc<ServerMessageHandler>) {
    info!("WebSocket client connected");

    // Handle the MCP initialization handshake
    let mut initialized = false;
    let mut buffer = BytesMut::with_capacity(1024);

    while let Some(result) = socket.recv().await {
        match result {
            Ok(msg) => {
                if let Ok(text) = msg.to_text() {
                    debug!("Received WebSocket message: {}", text);

                    // Parse and handle the MCP message
                    match json::from_str::<serde_json::Value>(text) {
                        Ok(json_msg) => {
                            let response_result = if !initialized {
                                // Handle initialization
                                if let Some(method) =
                                    json_msg.get("method").and_then(|m| m.as_str())
                                {
                                    if method == "initialize" {
                                        // Parse the initialize request
                                        match serde_json::from_value::<InitializeRequest>(
                                            json_msg.clone(),
                                        ) {
                                            Ok(init_request) => {
                                                match handler.handle_initialize(init_request).await
                                                {
                                                    Ok(init_response) => {
                                                        initialized = true;
                                                        Ok(Some(JsonRpcMessage::Response(
                                                            JsonRpcResponse {
                                                                jsonrpc: "2.0".to_string(),
                                                                id: serde_json::from_value(json_msg.get("id").cloned().unwrap_or(serde_json::Value::Null)).unwrap_or(RequestId::Null),
                                                                result: Some(serde_json::to_value(init_response).unwrap()),
                                                                error: None,
                                                            },
                                                        )))
                                                    }
                                                    Err(e) => Ok(Some(JsonRpcMessage::Response(
                                                        JsonRpcResponse {
                                                            jsonrpc: "2.0".to_string(),
                                                            id: serde_json::from_value(json_msg.get("id").cloned().unwrap_or(serde_json::Value::Null)).unwrap_or(RequestId::Null),
                                                            result: None,
                                                            error: Some(JsonRpcError {
                                                                code: -32603,
                                                                message: e.to_string(),
                                                                data: None,
                                                            }),
                                                        },
                                                    ))),
                                                }
                                            }
                                            Err(e) => Ok(Some(JsonRpcMessage::Response(
                                                JsonRpcResponse {
                                                    jsonrpc: "2.0".to_string(),
                                                    id: serde_json::from_value(json_msg.get("id").cloned().unwrap_or(serde_json::Value::Null)).unwrap_or(RequestId::Null),
                                                    result: None,
                                                    error: Some(JsonRpcError {
                                                        code: -32602,
                                                        message: format!("Invalid initialize request: {}", e),
                                                        data: None,
                                                    }),
                                                },
                                            ))),
                                        }
                                    } else {
                                        // Send error for non-initialize message before init
                                        Ok(Some(JsonRpcMessage::Response(JsonRpcResponse {
                                            jsonrpc: "2.0".to_string(),
                                            id: serde_json::from_value(json_msg.get("id").cloned().unwrap_or(serde_json::Value::Null)).unwrap_or(RequestId::Null),
                                            result: None,
                                            error: Some(JsonRpcError {
                                                code: -32002,
                                                message: "Server not initialized".to_string(),
                                                data: None,
                                            }),
                                        })))
                                    }
                                } else {
                                    Ok(Some(JsonRpcMessage::Response(JsonRpcResponse {
                                        jsonrpc: "2.0".to_string(),
                                        id: serde_json::from_value(json_msg.get("id").cloned().unwrap_or(serde_json::Value::Null)).unwrap_or(RequestId::Null),
                                        result: None,
                                        error: Some(JsonRpcError {
                                            code: -32600,
                                            message: "Invalid Request".to_string(),
                                            data: None,
                                        }),
                                    })))
                                }
                            } else {
                                // Handle regular MCP messages after initialization
                                handle_mcp_method(&handler, &json_msg).await
                            };

                            if let Ok(Some(response)) = response_result {
                                buffer.clear();
                                if let Err(e) =
                                    Protocol::serialize_message_to_buffer(&response, &mut buffer)
                                {
                                    error!("Failed to serialize response: {}", e);
                                    buffer.clear();
                                    let error_response = json!({
                                        "jsonrpc": "2.0",
                                        "error": {
                                            "code": -32603,
                                            "message": "Internal error"
                                        },
                                        "id": response.id()
                                    });
                                    serde_json::to_writer((&mut buffer).writer(), &error_response)
                                        .unwrap();
                                }

                                if let Err(e) = socket
                                    .send(axum::extract::ws::Message::Text(
                                        String::from_utf8_lossy(&buffer).to_string(),
                                    ))
                                    .await
                                {
                                    error!("Failed to send WebSocket response: {}", e);
                                    break;
                                }
                            } else if let Err(e) = response_result {
                                error!("Error handling message: {}", e);
                                let error_response = json!({
                                    "jsonrpc": "2.0",
                                    "error": {
                                        "code": -32603,
                                        "message": e.to_string()
                                    },
                                    "id": null
                                });
                                if let Err(e) = socket
                                    .send(axum::extract::ws::Message::Text(error_response.to_string()))
                                    .await
                                {
                                    error!("Failed to send error response: {}", e);
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse JSON message: {}", e);
                            let error_response = json!({
                                "jsonrpc": "2.0",
                                "error": {
                                    "code": -32700,
                                    "message": "Parse error"
                                },
                                "id": null
                            });
                            if let Err(e) = socket
                                .send(axum::extract::ws::Message::Text(error_response.to_string()))
                                .await
                            {
                                error!("Failed to send error response: {}", e);
                                break;
                            }
                        }
                    }
                } else {
                    warn!("Received non-text WebSocket message, ignoring");
                }
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
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
    axum::extract::State(_handler): axum::extract::State<Arc<ServerMessageHandler>>,
    axum::Json(request): axum::Json<serde_json::Value>,
) -> axum::Json<serde_json::Value> {
    // For now, return a simple response indicating HTTP support is available
    // This would need full protocol implementation similar to the WebSocket handler
    axum::Json(json!({
        "jsonrpc": "2.0",
        "error": {
            "code": -32601,
            "message": "HTTP transport not fully implemented yet - use WebSocket or stdio"
        },
        "id": request.get("id").cloned()
    }))
}
