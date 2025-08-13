# MoCoPr Architecture Guide

This guide provides an in-depth look at MoCoPr's architecture, design principles, and internal components.

## Table of Contents

1. [Overview](#overview)
2. [Design Principles](#design-principles)
3. [Core Architecture](#core-architecture)
4. [Package Structure](#package-structure)
5. [Protocol Implementation](#protocol-implementation)
6. [Transport Layer](#transport-layer)
7. [Server Architecture](#server-architecture)
8. [Client Architecture](#client-architecture)
9. [Extension Points](#extension-points)
10. [Security Architecture](#security-architecture)

## Overview {#overview}

MoCoPr (More Copper) is a comprehensive Rust implementation of the Model Context Protocol (MCP) specification. It's designed as a modular, high-performance framework that enables secure and efficient communication between AI models and external resources.

### Key Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Client    │    │   MCP Server    │    │  External App   │
│                 │    │                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │   Client    │◄┼────┼►│   Server    │◄┼────┼►│   Consumer  │ │
│ │    Core     │ │    │ │    Core     │ │    │ │    App      │ │
│ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │
│                 │    │                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │    │                 │
│ │ Transport   │ │    │ │ Transport   │ │    │                 │
│ │   Layer     │ │    │ │   Layer     │ │    │                 │
│ └─────────────┘ │    │ └─────────────┘ │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Design Principles {#design-principles}

### 1. Modularity and Composability

MoCoPr is built with a modular architecture where each component has a well-defined interface and can be composed with others:

```rust
// Components can be mixed and matched
let server = McpServerBuilder::new()
    .with_info("my-server", "1.0.0")
    .with_transport(CustomTransport::new())
    .with_middleware(AuthMiddleware::new())
    .with_tools()
    .build()?;
```

### 2. Type Safety

Extensive use of Rust's type system to prevent runtime errors:

```rust
// Compile-time guarantees about protocol compliance
pub struct JsonRpcRequest {
    pub jsonrpc: String,      // Must be "2.0"
    pub id: Option<RequestId>, // Strongly typed ID
    pub method: String,       // Validated method name
    pub params: Option<Value>,
}
```

### 3. Zero-Cost Abstractions

High-level APIs that compile down to efficient machine code:

```rust
// This high-level code compiles to efficient async state machines
#[async_trait]
impl ToolExecutor for MyTool {
    async fn execute(&self, args: Option<Value>) -> Result<ToolsCallResponse> {
        // Implementation
    }
}
```

### 4. Performance by Default

Built-in optimizations and performance-conscious design:

- Async/await throughout for non-blocking I/O
- Zero-copy deserialization where possible
- Connection pooling and resource reuse
- Efficient serialization formats

## Core Architecture {#core-architecture}

### Layered Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                        │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐ │
│  │    Tools    │ │  Resources  │ │       Prompts           │ │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                     Protocol Layer                          │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐ │
│  │   Request   │ │  Response   │ │     Notifications       │ │
│  │  Handlers   │ │ Builders    │ │      Handlers           │ │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                    Transport Layer                          │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐ │
│  │    Stdio    │ │  WebSocket  │ │         HTTP            │ │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                      Core Layer                             │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐ │
│  │    Types    │ │    Error    │ │       Utilities         │ │
│  │   System    │ │  Handling   │ │                         │ │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

```
Client Request → Transport → Protocol → Handler → Business Logic
                    ↑                      ↓
Client Response ← Transport ← Protocol ← Handler ← Business Logic
```

## Package Structure {#package-structure}

### mocopr-core

The foundation package containing core types and utilities:

```
mocopr-core/
├── src/
│   ├── lib.rs              # Public API exports
│   ├── error.rs            # Error types and handling
│   ├── types/              # Core MCP types
│   │   ├── mod.rs
│   │   ├── requests.rs     # Request types
│   │   ├── responses.rs    # Response types
│   │   ├── capabilities.rs # Capability definitions
│   │   └── common.rs       # Shared types
│   ├── protocol/           # Protocol implementation
│   │   ├── mod.rs
│   │   ├── jsonrpc.rs      # JSON-RPC implementation
│   │   └── validation.rs   # Message validation
│   ├── transport/          # Transport abstractions
│   │   ├── mod.rs
│   │   ├── traits.rs       # Transport traits
│   │   └── message.rs      # Transport messages
│   └── utils.rs            # Utility functions
└── tests/                  # Integration tests
```

### mocopr-server

High-level server API with builder patterns:

```
mocopr-server/
├── src/
│   ├── lib.rs              # Public API
│   ├── builder.rs          # Server builder
│   ├── server.rs           # Core server implementation
│   ├── handlers.rs         # Request handlers
│   ├── registry.rs         # Component registries
│   └── middleware.rs       # Middleware support
└── examples/               # Usage examples
```

### mocopr-client

Client library for connecting to MCP servers:

```
mocopr-client/
├── src/
│   ├── lib.rs              # Client API
│   ├── client.rs           # Core client implementation
│   ├── connection.rs       # Connection management
│   └── builder.rs          # Client builder
└── tests/                  # Client tests
```

### mocopr-macros

Procedural macros for reducing boilerplate:

```
mocopr-macros/
├── src/
│   ├── lib.rs              # Macro exports
│   ├── tool.rs             # Tool derive macro
│   ├── resource.rs         # Resource derive macro
│   └── prompt.rs           # Prompt derive macro
└── tests/                  # Macro tests
```

## Protocol Implementation {#protocol-implementation}

### JSON-RPC 2.0 Foundation

MCP is built on JSON-RPC 2.0, with MoCoPr providing a complete implementation:

```rust
// Core JSON-RPC types
pub struct JsonRpcRequest {
    pub jsonrpc: String,        // Always "2.0"
    pub id: Option<RequestId>,  // Request identifier
    pub method: String,         // Method name
    pub params: Option<Value>,  // Method parameters
}

pub struct JsonRpcResponse {
    pub jsonrpc: String,        // Always "2.0"
    pub id: Option<RequestId>,  // Matches request ID
    pub result: Option<Value>,  // Success result
    pub error: Option<JsonRpcError>, // Error information
}
```

### MCP Method Mapping

MCP defines specific methods that map to JSON-RPC calls:

```rust
// Method routing based on MCP specification
match request.method.as_str() {
    "initialize" => self.handle_initialize(request).await,
    "tools/list" => self.handle_tools_list(request).await,
    "tools/call" => self.handle_tools_call(request).await,
    "resources/list" => self.handle_resources_list(request).await,
    "resources/read" => self.handle_resources_read(request).await,
    "prompts/list" => self.handle_prompts_list(request).await,
    "prompts/get" => self.handle_prompts_get(request).await,
    _ => Err(Error::method_not_found(&request.method)),
}
```

### Capability Negotiation

MCP uses capability negotiation to determine what features are supported:

```rust
#[derive(Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub resources: Option<ResourcesCapability>,
    pub tools: Option<ToolsCapability>,
    pub prompts: Option<PromptsCapability>,
    pub logging: Option<LoggingCapability>,
}

impl ServerCapabilities {
    pub fn with_tools(mut self, list_changed: bool) -> Self {
        self.tools = Some(ToolsCapability { list_changed });
        self
    }
}
```

## Transport Layer {#transport-layer}

### Transport Abstraction

All transports implement a common trait:

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, message: TransportMessage) -> Result<()>;
    async fn receive(&self) -> Result<TransportMessage>;
    async fn close(&self) -> Result<()>;
}
```

### Stdio Transport

Direct stdin/stdout communication:

```rust
pub struct StdioTransport {
    stdin: Mutex<BufReader<Stdin>>,
    stdout: Mutex<BufWriter<Stdout>>,
}

impl StdioTransport {
    // Reads JSON-RPC messages from stdin
    async fn read_message(&self) -> Result<String> {
        let mut line = String::new();
        self.stdin.lock().await.read_line(&mut line).await?;
        Ok(line)
    }
}
```

### WebSocket Transport

Full-duplex WebSocket communication:

```rust
pub struct WebSocketTransport {
    ws_stream: WebSocketStream<TcpStream>,
    send_queue: UnboundedSender<Message>,
}

impl WebSocketTransport {
    pub async fn connect(url: &str) -> Result<Self> {
        let (ws_stream, _) = connect_async(url).await?;
        // Setup bidirectional message handling
        Ok(Self { ws_stream, send_queue })
    }
}
```

## Server Architecture {#server-architecture}

### Server Lifecycle

```rust
pub struct McpServer {
    capabilities: ServerCapabilities,
    tool_registry: ToolRegistry,
    resource_registry: ResourceRegistry,
    prompt_registry: PromptRegistry,
    middleware_stack: MiddlewareStack,
}

impl McpServer {
    pub async fn run_stdio(self) -> Result<()> {
        let transport = StdioTransport::new();
        self.run_with_transport(transport).await
    }

    async fn run_with_transport<T: Transport>(
        self,
        transport: T,
    ) -> Result<()> {
        loop {
            let message = transport.receive().await?;
            let response = self.handle_message(message).await?;
            transport.send(response).await?;
        }
    }
}
```

### Request Processing Pipeline

```rust
async fn handle_message(&self, message: TransportMessage) -> Result<TransportMessage> {
    // 1. Parse JSON-RPC
    let request: JsonRpcRequest = serde_json::from_slice(&message.data)?;

    // 2. Validate request
    self.validate_request(&request)?;

    // 3. Apply middleware (before)
    let mut request = self.middleware_stack.before_request(request).await?;

    // 4. Route to handler
    let mut response = self.route_request(request).await?;

    // 5. Apply middleware (after)
    response = self.middleware_stack.after_request(request, response).await?;

    // 6. Serialize response
    let response_data = serde_json::to_vec(&response)?;
    Ok(TransportMessage::new(response_data))
}
```

### Registry System

Component registration and discovery:

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn ToolHandler>>,
    metadata: HashMap<String, ToolMetadata>,
}

impl ToolRegistry {
    pub fn register<T: ToolHandler + 'static>(&mut self, tool: T) {
        let metadata = tool.metadata();
        self.tools.insert(metadata.name.clone(), Box::new(tool));
        self.metadata.insert(metadata.name.clone(), metadata);
    }

    pub async fn execute(&self, name: &str, args: Option<Value>) -> Result<ToolsCallResponse> {
        let tool = self.tools.get(name)
            .ok_or_else(|| Error::tool_not_found(name))?;
        tool.execute(args).await
    }
}
```

## Client Architecture {#client-architecture}

### Client Lifecycle

```rust
pub struct McpClient {
    transport: Box<dyn Transport>,
    request_id: AtomicU64,
    pending_requests: Arc<Mutex<HashMap<RequestId, oneshot::Sender<JsonRpcResponse>>>>,
}

impl McpClient {
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<ToolsCallResponse> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(self.next_request_id()),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": name,
                "arguments": arguments
            })),
        };

        let response = self.send_request(request).await?;
        Ok(serde_json::from_value(response.result.unwrap())?)
    }
}
```

### Connection Management

```rust
impl McpClient {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let (tx, rx) = oneshot::channel();

        // Store pending request
        if let Some(id) = &request.id {
            self.pending_requests.lock().await.insert(id.clone(), tx);
        }

        // Send request
        let message = TransportMessage::new(serde_json::to_vec(&request)?);
        self.transport.send(message).await?;

        // Wait for response
        rx.await?
    }

    async fn handle_incoming_messages(&self) {
        while let Ok(message) = self.transport.receive().await {
            if let Ok(response) = serde_json::from_slice::<JsonRpcResponse>(&message.data) {
                if let Some(id) = &response.id {
                    if let Some(tx) = self.pending_requests.lock().await.remove(id) {
                        let _ = tx.send(response);
                    }
                }
            }
        }
    }
}
```

## Extension Points {#extension-points}

### Custom Tools

Implement the `ToolHandler` trait:

```rust
pub struct CustomTool {
    config: CustomConfig,
}

#[async_trait]
impl ToolHandler for CustomTool {
    async fn execute(&self, arguments: Option<Value>) -> Result<ToolsCallResponse> {
        // Custom tool implementation
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "custom_tool".to_string(),
            description: "A custom tool implementation".to_string(),
            input_schema: self.input_schema(),
        }
    }
}
```

### Custom Resources

Implement the `ResourceHandler` trait:

```rust
pub struct CustomResource {
    data_source: DataSource,
}

#[async_trait]
impl ResourceHandler for CustomResource {
    async fn read(&self, uri: &str) -> Result<ResourcesReadResponse> {
        // Custom resource reading logic
    }

    async fn list(&self) -> Result<ResourcesListResponse> {
        // List available resources
    }
}
```

### Custom Middleware

Implement middleware traits:

```rust
#[async_trait]
impl RequestMiddleware for CustomMiddleware {
    async fn before_request(&self, request: &mut JsonRpcRequest) -> Result<()> {
        // Modify request before processing
    }

    async fn after_request(&self, request: &JsonRpcRequest, response: &mut JsonRpcResponse) -> Result<()> {
        // Modify response after processing
    }
}
```

## Security Architecture {#security-architecture}

### Input Validation

All inputs are validated at multiple layers:

```rust
pub struct RequestValidator;

impl RequestValidator {
    pub fn validate_request(request: &JsonRpcRequest) -> Result<()> {
        // Validate JSON-RPC structure
        if request.jsonrpc != "2.0" {
            return Err(Error::invalid_request("Invalid JSON-RPC version"));
        }

        // Validate method name
        if !Self::is_valid_method(&request.method) {
            return Err(Error::method_not_found(&request.method));
        }

        // Validate parameters
        if let Some(params) = &request.params {
            Self::validate_parameters(&request.method, params)?;
        }

        Ok(())
    }
}
```

### Authentication and Authorization

Pluggable security through middleware:

```rust
pub struct AuthMiddleware {
    authenticator: Box<dyn Authenticator>,
    authorizer: Box<dyn Authorizer>,
}

#[async_trait]
impl RequestMiddleware for AuthMiddleware {
    async fn before_request(&self, request: &mut JsonRpcRequest) -> Result<()> {
        // Extract credentials
        let credentials = self.extract_credentials(request)?;

        // Authenticate user
        let user = self.authenticator.authenticate(credentials).await?;

        // Authorize request
        self.authorizer.authorize(&user, &request.method).await?;

        // Store user context
        request.set_metadata("user", user);
        Ok(())
    }
}
```

### Secure Defaults

MoCoPr implements secure defaults:

- All network communication can use TLS
- Input validation is mandatory
- Resource access is sandboxed by default
- Rate limiting is built-in
- Audit logging is supported

## Performance Characteristics

### Async-First Design

Built on Tokio for high-performance async I/O:

```rust
// Non-blocking request handling
async fn handle_concurrent_requests(&self, requests: Vec<JsonRpcRequest>) -> Vec<JsonRpcResponse> {
    let tasks: Vec<_> = requests.into_iter()
        .map(|req| self.handle_request(req))
        .collect();

    futures::future::join_all(tasks).await
}
```

### Zero-Copy Optimizations

Minimize data copying where possible:

```rust
// Use Bytes for zero-copy operations
pub struct TransportMessage {
    pub data: Bytes,  // Reference-counted byte buffer
    pub metadata: HashMap<String, String>,
}
```

### Connection Pooling

Built-in connection pooling for client connections:

```rust
pub struct ConnectionPool {
    pool: deadpool::Pool<McpClient>,
    config: PoolConfig,
}
```

This architecture provides a solid foundation for building scalable, secure, and high-performance MCP applications while maintaining flexibility and extensibility.
