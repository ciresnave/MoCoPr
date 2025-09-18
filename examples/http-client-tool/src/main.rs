use anyhow::Result;
use mocopr_core::ToolExecutor;
use mocopr_macros::Tool;
use mocopr_server::prelude::*;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

/// A tool that fetches the content of a URL.
#[derive(Tool)]
#[tool(name = "http_get", description = "Fetches the content of a URL.")]
struct HttpGetTool {
    /// A shared reqwest client for making HTTP requests.
    client: Arc<Client>,
}

impl HttpGetTool {
    /// Creates a new `HttpGetTool` with a shared `reqwest::Client`.
    fn new(client: Arc<Client>) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl ToolExecutor for HttpGetTool {
    /// Executes the tool to fetch the content of a URL.
    async fn execute(
        &self,
        arguments: Option<Value>,
    ) -> mocopr_core::Result<ToolsCallResponse> {
        let args = arguments.unwrap_or_default();
        let url = match args.get("url").and_then(|v| v.as_str()) {
            Some(url) => url,
            None => {
                return Ok(ToolsCallResponse::error(vec![Content::Text(
                    TextContent::new("Missing required parameter: url"),
                )]));
            }
        };

        info!("Fetching URL: {}", url);

        match self.client.get(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.text().await {
                        Ok(text) => Ok(ToolsCallResponse::success(vec![Content::Text(
                            TextContent::new(&text),
                        )])),
                        Err(e) => Ok(ToolsCallResponse::error(vec![Content::Text(
                            TextContent::new(&format!("Failed to read response text: {}", e)),
                        )])),
                    }
                } else {
                    Ok(ToolsCallResponse::error(vec![Content::Text(
                        TextContent::new(&format!("Request failed with status: {}", response.status())),
                    )]))
                }
            }
            Err(e) => Ok(ToolsCallResponse::error(vec![Content::Text(
                TextContent::new(&format!("Failed to send request: {}", e)),
            )])),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create a single reqwest::Client to be shared across all tools.
    let client = Arc::new(Client::new());

    // Create the tool
    let http_get_tool = HttpGetTool::new(client);

    // Build and start the server
    let server = McpServerBuilder::new()
        .with_info("HTTP Client Tool Server", "1.0.0")
        .with_tools()
        .with_tool(http_get_tool)
        .build()?;

    info!("MCP HTTP Client Tool Server ready. Capabilities:");
    info!("- Tools: http_get");

    // Run the server using stdio transport
    server.run_stdio().await?;

    Ok(())
}
