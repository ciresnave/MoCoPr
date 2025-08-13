# Quick Start Guide

This guide will get you up and running with MoCoPr in under 5 minutes.

## What is MoCoPr?

MoCoPr (More Copper) is a comprehensive Rust implementation of the Model Context Protocol (MCP). It provides:

- **High-performance** async MCP server and client implementations
- **Type-safe** APIs with procedural macros
- **Multi-transport** support (stdio, WebSocket, HTTP)
- **Production-ready** security and monitoring features
- **Zero-copy** serialization for optimal performance

## Prerequisites

- Rust 1.70+ installed
- Basic knowledge of Rust and async programming

## Creating Your First MCP Server

### 1. Create a new Rust project

```bash
cargo new my-mcp-server
cd my-mcp-server
```

### 2. Add MoCoPr dependencies

```toml
[dependencies]
mocopr-server = "0.1.0"
mocopr-core = "0.1.0"
mocopr-macros = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
async-trait = "0.1"
```

### 3. Implement a simple calculator tool

```rust
use mocopr_server::prelude::*;
use mocopr_macros::Tool;
use mocopr_core::ToolExecutor;
use serde_json::{json, Value};
use std::collections::HashMap;
use anyhow::Result;

#[derive(Tool)]
#[tool(name = "calculator", description = "Basic arithmetic calculator")]
struct Calculator;

impl Calculator {
    async fn execute_impl(&self, params: Value) -> anyhow::Result<Value> {
        let operation = params["operation"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing operation parameter"))?;

        let a = params["a"]
            .as_f64()
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid parameter 'a'"))?;

        let b = params["b"]
            .as_f64()
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid parameter 'b'"))?;

        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => {
                if b == 0.0 {
                    return Err(anyhow::anyhow!("Division by zero"));
                }
                a / b
            }
            _ => return Err(anyhow::anyhow!("Unsupported operation: {}", operation)),
        };

        Ok(json!({ "result": result }))
    }
}

#[async_trait::async_trait]
impl ToolExecutor for Calculator {
    async fn execute(
        &self,
        arguments: Option<serde_json::Value>,
    ) -> mocopr_core::Result<mocopr_core::types::ToolsCallResponse> {
        let args = arguments.unwrap_or_default();
        match self.execute_impl(args).await {
            Ok(result) => Ok(mocopr_core::types::ToolsCallResponse::success(vec![
                mocopr_core::types::Content::Text(mocopr_core::types::TextContent::new(
                    result.to_string(),
                )),
            ])),
            Err(e) => Ok(mocopr_core::types::ToolsCallResponse::error(vec![
                mocopr_core::types::Content::Text(mocopr_core::types::TextContent::new(
                    e.to_string(),
                )),
            ])),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Build the server
    let server = McpServerBuilder::new()
        .with_info("calculator-server", "1.0.0")
        .with_description("A simple calculator MCP server")
        .with_tools(vec![Box::new(Calculator)])
        .build()?;

    // Run the server on stdio
    println!("Calculator MCP server starting...");
    server.run_stdio().await?;

    Ok(())
}
```

### 4. Run your server

```bash
cargo run
```

Your MCP server is now running and ready to accept connections!

## Testing Your Server

### Using the Simple Client

Create a test client to verify your server works:

```rust
use mocopr_client::prelude::*;
use serde_json::json;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to the server
    let client = McpClient::connect_stdio(
        "cargo",
        &["run", "--bin", "my-mcp-server"],
        Implementation {
            name: "Test Client".to_string(),
            version: "1.0.0".to_string(),
        },
        ClientCapabilities::default(),
    ).await?;

    // Call the calculator tool
    let result = client.call_tool("calculator", json!({
        "operation": "add",
        "a": 5.0,
        "b": 3.0
    })).await?;

    println!("5 + 3 = {}", result["result"]);

    Ok(())
}
```

## Next Steps

- [Building Your First Server](02-building-your-first-server.md) - Comprehensive server development guide
- [Advanced Features](03-advanced-features.md) - Resources, prompts, and middleware
- [Production Deployment](04-production-deployment.md) - Deploy to production environments
- [Performance Tuning](05-performance-tuning.md) - Optimize for high-performance scenarios

## Common Issues

### "command not found" errors

Make sure Rust and Cargo are properly installed and in your PATH.

### Compilation errors

Ensure you're using Rust 1.70+ and all dependencies are correctly specified.

### Connection issues

Check that both client and server are using compatible transport methods.

## Getting Help

- 📚 [Full Documentation](https://docs.rs/mocopr)
- 🐛 [Report Issues](https://github.com/cires-ai/mocopr/issues)
- 💬 [GitHub Discussions](https://github.com/cires-ai/mocopr/discussions)
- 📧 Email: <ciresnave@gmail.com>
