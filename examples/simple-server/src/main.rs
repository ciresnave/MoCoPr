//! Simple MCP Server Example
//!
//! This example demonstrates how to create a basic MCP server with
//! resources, tools, and prompts using MoCoPr.

use mocopr_server::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create server builder
    let server = McpServer::builder()
        .with_info("Simple MCP Server", "1.0.0")
        .with_logging()
        .with_resources()
        .with_tools()
        .with_prompts()
        .build()?;

    println!("Starting simple MCP server...");
    println!("Use this server with an MCP client via stdio");

    // Run the server
    server.run_stdio().await?;

    Ok(())
}
