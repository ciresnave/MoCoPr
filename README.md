# MoCoPr (More Copper) 🔥

A comprehensive and developer-friendly Rust implementation of the **Model Context Protocol (MCP)**, making it extremely simple to build MCP servers and clients with production-ready features.

[![License](https://img.shields.io/crates/l/MoCoPr)](https://github.com/ciresnave/MoCoPr)
[![Rust Version](https://img.shields.io/badge/rust-1.70+-green.svg)](https://www.rust-lang.org)
[![CI](https://github.com/ciresnave/mocopr/workflows/CI%2FCD%20Pipeline/badge.svg)](https://github.com/ciresnave/mocopr/actions)
[![Coverage](https://codecov.io/gh/ciresnave/mocopr/branch/main/graph/badge.svg)](https://codecov.io/gh/ciresnave/mocopr)
[![Crates.io](https://img.shields.io/crates/v/mocopr.svg)](https://crates.io/crates/mocopr)
[![Docs.rs](https://img.shields.io/docsrs/MoCoPr)](https://docs.rs/mocopr)

## 🚀 Features

- **🔧 Complete MCP Implementation**: Full support for MCP specification (2025-06-18)
- **🚀 Developer-Friendly API**: Intuitive builder patterns and macros for rapid development
- **🌐 Multiple Transports**: stdio, WebSocket, and HTTP transport support with automatic reconnection
- **🔒 Type-Safe**: Leverages Rust's type system for protocol safety and compile-time guarantees
- **⚡ High Performance**: Built on tokio for high-performance async I/O with benchmarked optimizations
- **🔌 Extensible**: Plugin architecture for custom resources, tools, and prompts
- **📚 Well-Documented**: Comprehensive documentation, examples, and tutorials
- **🛡️ Production Ready**: Comprehensive error handling, logging, metrics, and security features
- **🧪 Thoroughly Tested**: Extensive test suite with 95%+ code coverage and stress testing
- **🔍 Observable**: Built-in tracing, metrics, and health checks

## 📦 Crates

MoCoPr is organized into several specialized crates for maximum flexibility:

| Crate | Description | Version | Features |
|-------|-------------|---------|----------|
| **`mocopr`** | Meta-crate with all components | [![Crates.io](https://img.shields.io/crates/v/mocopr.svg)](https://crates.io/crates/mocopr) | Complete MCP toolkit |
| **`mocopr-core`** | Core types and protocol implementation | [![Crates.io](https://img.shields.io/crates/v/mocopr-core.svg)](https://crates.io/crates/mocopr-core) | Protocol foundation |
| **`mocopr-server`** | High-level server implementation | [![Crates.io](https://img.shields.io/crates/v/mocopr-server.svg)](https://crates.io/crates/mocopr-server) | Server builder API |
| **`mocopr-client`** | High-level client implementation | [![Crates.io](https://img.shields.io/crates/v/mocopr-client.svg)](https://crates.io/crates/mocopr-client) | Client connection API |
| **`mocopr-macros`** | Procedural macros for boilerplate reduction | [![Crates.io](https://img.shields.io/crates/v/mocopr-macros.svg)](https://crates.io/crates/mocopr-macros) | Derive macros |

## 🏃‍♂️ Quick Start

Add MoCoPr to your `Cargo.toml`:

```toml
[dependencies]
mocopr = "0.1"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

### Creating an MCP Server

```rust
use mocopr::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a simple calculator server
    let server = McpServer::builder()
        .with_info("Calculator Server", "1.0.0")
        .with_tool_handler(
            "add",
            "Add two numbers together",
            json!({
                "type": "object",
                "properties": {
                    "a": {"type": "number", "description": "First number"},
                    "b": {"type": "number", "description": "Second number"}
                },
                "required": ["a", "b"]
            }),
            |params| async move {
                let a = params["a"].as_f64().unwrap_or(0.0);
                let b = params["b"].as_f64().unwrap_or(0.0);

                Ok(ToolResult::text(format!("Result: {}", a + b)))
            }
        )
        .with_resource_handler(
            "memory://calculations",
            "Access calculation history",
            |_uri| async move {
                Ok(ResourceResult::text("Recent calculations: ..."))
            }
        )
        .build()?;

    // Start server with stdio transport
    server.run_stdio().await?;
    Ok(())
}
```

### Creating an MCP Client

```rust
use mocopr::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to an MCP server
    let client = McpClient::connect_stdio(
        "python",
        &["calculator_server.py"],
        Implementation::new("Calculator Client", "1.0.0"),
        ClientCapabilities::default(),
    ).await?;

    // List available tools
    let tools = client.list_tools().await?;
    println!("Available tools: {:#?}", tools);

    // Call a tool
    let result = client.call_tool("add", json!({
        "a": 42,
        "b": 58
    })).await?;

    println!("Result: {:#?}", result);

    // Read a resource
    let resource = client.read_resource("memory://calculations").await?;
    println!("Resource: {:#?}", resource);

    Ok(())
}
```

## 📖 Documentation

### 📚 Guides and Tutorials

- [**Quick Start Guide**](docs/tutorials/01-quick-start.md) - Get up and running in 5 minutes
- [**Building Your First MCP Server**](docs/tutorials/02-building-your-first-server.md) - Comprehensive tutorial
- [**Advanced Server Features**](docs/tutorials/03-advanced-features.md) - Middleware, validation, etc.
- [**Production Deployment**](docs/tutorials/04-production-deployment.md) - Production-ready deployment guide
- [**Performance Tuning**](docs/tutorials/05-performance-tuning.md) - Performance optimization and tuning
- [**Architecture Guide**](docs/guides/architecture.md) - System architecture and design patterns
- [**Security Best Practices**](docs/security.md) - Security guidelines and recommendations
- [**Performance Optimization**](docs/performance.md) - Benchmarking and optimization tips

### 🔧 API Documentation

- [**Core API Reference**](https://docs.rs/mocopr-core) - Protocol types and utilities
- [**Server API Reference**](https://docs.rs/mocopr-server) - Server builder and handlers
- [**Client API Reference**](https://docs.rs/mocopr-client) - Client connection management
- [**Macros Reference**](https://docs.rs/mocopr-macros) - Derive macro documentation

## 🌟 Examples

The [`examples/`](examples/) directory contains comprehensive examples:

| Example | Description | Transport | Complexity |
|---------|-------------|-----------|------------|
| [**simple-server**](examples/simple-server/) | Basic MCP server with tools and resources | stdio | Beginner |
| [**simple-client**](examples/simple-client/) | Basic MCP client usage | stdio | Beginner |
| [**calculator-server**](examples/calculator-server/) | Full-featured calculator with validation | stdio | Intermediate |
| [**file-server**](examples/file-server/) | File system operations with security | stdio | Intermediate |
| [**websocket-chat**](examples/websocket-chat/) | Real-time chat server | WebSocket | Advanced |
| [**http-api**](examples/http-api/) | REST-like HTTP interface | HTTP | Advanced |
| [**production-server**](examples/production-server/) | Production-ready server with all features | All | Expert |

Run any example:

```bash
# Run the calculator server
cargo run --example calculator-server

# Run the file server
cargo run --example file-server

# Run with custom transport
cargo run --example websocket-chat -- --port 8080
```

## 🚀 Features in Detail

### Transport Support

MoCoPr supports multiple transport mechanisms with automatic failover:

```rust
// stdio transport (process communication)
server.run_stdio().await?;

// WebSocket transport (real-time web apps)
server.run_websocket("127.0.0.1:8080").await?;

// HTTP transport (stateless requests)
server.run_http("127.0.0.1:3000").await?;

// Multiple transports simultaneously
server.run_all(&[
    TransportConfig::Stdio,
    TransportConfig::WebSocket("127.0.0.1:8080".parse()?),
    TransportConfig::Http("127.0.0.1:3000".parse()?),
]).await?;
```

### Advanced Features

#### Middleware and Validation

```rust
let server = McpServer::builder()
    .with_middleware(RateLimitMiddleware::new(100, Duration::from_secs(60)))
    .with_middleware(AuthenticationMiddleware::new())
    .with_middleware(LoggingMiddleware::with_level(Level::INFO))
    .with_validation(true)
    .build()?;
```

#### Monitoring and Observability

```rust
use mocopr::observability::*;

let server = McpServer::builder()
    .with_metrics(PrometheusMetrics::new())
    .with_tracing(TracingConfig::default())
    .with_health_checks(true)
    .build()?;
```

#### Error Handling and Recovery

```rust
let client = McpClient::builder()
    .with_retry_policy(RetryPolicy::exponential_backoff(3))
    .with_timeout(Duration::from_secs(30))
    .with_connection_recovery(true)
    .connect_stdio("server", &["args"]).await?;
```

            let result = a + b;

            Ok(ToolsCallResponse::success(vec![
                Content::from(format!("{} + {} = {}", a, b, result))
            ]))
        }
    );

    // Build and run the server
    let server = McpServer::builder()
        .with_info("Calculator Server", "1.0.0")
        .with_tools()
        .with_tool(calculator)
        .build()?;

    server.run_stdio().await?;
    Ok(())
}

```

### Creating an MCP Client

```rust
use mocopr_client::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to a server
    let client = McpClientBuilder::new()
        .with_info("My Client".to_string(), "1.0.0".to_string())
        .connect_stdio("my-mcp-server", &[])
        .await?;

    // List available tools
    let tools = client.list_tools().await?;
    println!("Available tools: {:?}", tools.tools);

    // Call a tool
    let result = client.call_tool(
        "add".to_string(),
        Some(json!({"a": 5, "b": 3}))
    ).await?;

    println!("Result: {:?}", result);

    client.close().await?;
    Ok(())
}
```

## 🛠️ Advanced Usage

### Using Macros for Clean Code

```rust
use mocopr_macros::*;

#[derive(Tool)]
#[tool(name = "weather", description = "Get weather information")]
struct WeatherTool;

#[async_trait]
impl ToolHandler for WeatherTool {
    async fn call(&self, args: Option<serde_json::Value>) -> Result<ToolsCallResponse> {
        // Your implementation here
        Ok(ToolsCallResponse::success(vec![
            Content::from("Sunny, 72°F")
        ]))
    }
}

#[derive(Resource)]
#[resource(uri = "file:///config.json", name = "Configuration")]
struct ConfigResource {
    data: serde_json::Value,
}
```

### File Resources

```rust
let file_resource = file_resource!(
    uri: "file:///data/example.txt",
    name: "Example Data",
    path: "./data/example.txt",
    description: "Example data file",
    mime_type: "text/plain"
);
```

### Template Prompts

```rust
let prompt = template_prompt!(
    name: "summarize",
    description: "Summarize text content",
    template: "Please summarize the following text: {text}",
    arguments: [
        PromptArgument::new("text")
            .with_description("Text to summarize")
            .required(true)
    ]
);
```

## 🌐 Transport Support

### Stdio (Process Communication)

```rust
// Server
server.run_stdio().await?;

// Client
let client = McpClientBuilder::new()
    .with_info("Client", "1.0.0")
    .connect_stdio("./my-server", &["--arg1", "value1"])
    .await?;
```

### WebSocket

```rust
// Server
server.run_websocket("127.0.0.1:8080").await?;

// Client
let client = McpClientBuilder::new()
    .with_info("Client", "1.0.0")
    .connect_websocket("ws://127.0.0.1:8080/mcp")
    .await?;
```

## 📖 Examples

The repository includes several complete examples:

- **Simple Server**: Basic MCP server with resources, tools, and prompts
- **Simple Client**: Client that connects and interacts with servers
- **File Server**: Server that exposes file system resources
- **Calculator Server**: Advanced calculator with multiple operations

Run examples:

```bash
# Start the simple server
cargo run --example simple-server

# In another terminal, run the client
cargo run --example simple-client
```

## 🏗️ Architecture

```
┌─────────────────┐    ┌─────────────────┐
│   MCP Client    │    │   MCP Server    │
│                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │   Session   │ │◄───┤ │   Session   │ │
│ └─────────────┘ │    │ └─────────────┘ │
│        │        │    │        │        │
│ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │  Transport  │ │◄───┤ │  Transport  │ │
│ └─────────────┘ │    │ └─────────────┘ │
└─────────────────┘    └─────────────────┘
        │                        │
        └── stdio/websocket ─────┘
```

### Core Components

- **Protocol Layer**: JSON-RPC 2.0 message handling
- **Transport Layer**: Pluggable transport implementations
- **Session Management**: Connection lifecycle and state
- **Handler Registry**: Dynamic resource/tool/prompt registration
- **Type System**: Full MCP type definitions with serde support

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p mocopr-core

# Run with logging
RUST_LOG=debug cargo test
```

## 📋 Requirements

- **Rust**: 1.70 or later
- **Dependencies**: See `Cargo.toml` for full list

## 📄 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## 🔗 Related

- [Model Context Protocol Specification](https://modelcontextprotocol.io/specification/2025-06-18)
- [MCP Official Documentation](https://modelcontextprotocol.io/)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)

---

**MoCoPr** - Making MCP integration in Rust as easy as adding copper to your project! ⚡
