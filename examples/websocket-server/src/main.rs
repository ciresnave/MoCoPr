#![allow(dead_code)]

use anyhow::Result;
use mocopr_core::{ResourceReader, ToolExecutor};
use mocopr_macros::{Resource, Tool};
use mocopr_server::prelude::*;
use serde_json::{json, Value};
use tracing::info;

/// A simple tool for WebSocket testing
#[derive(Tool)]
#[tool(name = "echo", description = "Echo back the input message")]
struct EchoTool;

impl EchoTool {
    fn new() -> Self {
        Self
    }

    async fn execute_impl(&self, args: Value) -> Result<Value> {
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("No message provided");

        Ok(json!({
            "echo": message,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

#[async_trait]
impl ToolExecutor for EchoTool {
    async fn execute(
        &self,
        arguments: Option<Value>,
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

/// A simple resource for WebSocket testing
#[derive(Resource)]
#[resource(name = "status", description = "Get server status information")]
struct StatusResource;

impl StatusResource {
    fn new() -> Self {
        Self
    }

    async fn execute_impl(&self, _uri: String) -> Result<String> {
        let result = json!({
            "server": "WebSocket MCP Server",
            "status": "running",
            "protocol": "MCP over WebSocket",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        Ok(result.to_string())
    }
}

#[async_trait]
impl ResourceReader for StatusResource {
    async fn read_resource(&self) -> mocopr_core::Result<Vec<mocopr_core::types::ResourceContent>> {
        match self.execute_impl("status://server".to_string()).await {
            Ok(result) => {
                let uri = url::Url::parse("status://server").unwrap();
                Ok(vec![mocopr_core::types::ResourceContent::new(
                    uri,
                    vec![mocopr_core::types::Content::Text(
                        mocopr_core::types::TextContent::new(result),
                    )],
                )])
            }
            Err(e) => {
                let uri = url::Url::parse("status://server").unwrap();
                Ok(vec![mocopr_core::types::ResourceContent::new(
                    uri,
                    vec![mocopr_core::types::Content::Text(
                        mocopr_core::types::TextContent::new(format!("Error: {}", e)),
                    )],
                )])
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting WebSocket MCP Server");

    let server = McpServerBuilder::new()
        .with_info("WebSocket MCP Server", "1.0.0")
        .with_logging()
        .with_resources()
        .with_tools()
        .with_prompts()
        .with_tool(EchoTool::new())
        .with_resource(StatusResource::new())
        .build()?;

    let addr = "127.0.0.1:8080";
    info!("WebSocket server will be available at: ws://{}/mcp", addr);
    info!("You can test it using a WebSocket client or the MoCoPr WebSocket transport");

    server.run_websocket(addr).await?;

    Ok(())
}
