use anyhow::{anyhow, Result};
use mocopr_core::ToolExecutor;
use mocopr_macros::Tool;
use mocopr_server::prelude::*;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use tracing::info;
use url::Url;

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

    /// Validates the URL format.
    fn validate_url(url: &str) -> Result<()> {
        Url::parse(url)
            .map(|_| ())
            .map_err(|e| anyhow!("Invalid URL format: {}", e))
    }
}

#[async_trait::async_trait]
impl ToolExecutor for HttpGetTool {
    /// Executes the tool to fetch the content of a URL.
    async fn execute(&self, arguments: Option<Value>) -> mocopr_core::Result<ToolsCallResponse> {
        let args = arguments.unwrap_or_default();
        let url = match args.get("url").and_then(|v| v.as_str()) {
            Some(url) => url,
            None => {
                return Ok(ToolsCallResponse::error(vec![Content::StructuredError(
                    StructuredErrorContent::new(
                        "missing_parameter",
                        "Missing required parameter: url",
                        None,
                    ),
                )]));
            }
        };

        if let Err(e) = Self::validate_url(url) {
            return Ok(ToolsCallResponse::error(vec![Content::StructuredError(
                StructuredErrorContent::new("invalid_url", e.to_string(), None),
            )]));
        }

        info!("Fetching URL: {}", url);

        match self.client.get(url).send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    match response.text().await {
                        Ok(text) => Ok(ToolsCallResponse::success(vec![Content::Text(
                            TextContent::new(&text),
                        )])),
                        Err(e) => Ok(ToolsCallResponse::error(vec![Content::StructuredError(
                            StructuredErrorContent::new(
                                "read_response_text_error",
                                &format!("Failed to read response text: {}", e),
                                Some(status.as_u16()),
                            ),
                        )])),
                    }
                } else {
                    Ok(ToolsCallResponse::error(vec![Content::StructuredError(
                        StructuredErrorContent::new(
                            "http_error",
                            &format!("Request failed with status: {}", status),
                            Some(status.as_u16()),
                        ),
                    )]))
                }
            }
            Err(e) => Ok(ToolsCallResponse::error(vec![Content::StructuredError(
                StructuredErrorContent::new(
                    "request_failed",
                    &format!("Failed to send request: {}", e),
                    None,
                ),
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
