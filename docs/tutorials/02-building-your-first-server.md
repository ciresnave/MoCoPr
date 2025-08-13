# Building Your First MCP Server

This comprehensive guide walks you through building a production-quality MCP server with MoCoPr.

## Overview

In this tutorial, you'll learn:

- Server architecture and design patterns
- Implementing tools, resources, and prompts
- Adding middleware for logging and security
- Error handling best practices
- Testing your server

## Project Setup

### 1. Initialize the Project

```bash
cargo new --bin mcp-file-server
cd mcp-file-server
```

### 2. Configure Dependencies

Add this to your `Cargo.toml`:

```toml
[package]
name = "mcp-file-server"
version = "0.1.0"
edition = "2021"

[dependencies]
mocopr-server = "0.1.0"
mocopr-core = "0.1.0"
mocopr-macros = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1.0", features = ["v4"] }
clap = { version = "4.0", features = ["derive"] }
async-trait = "0.1"
```

## Building the File Server

### 1. Define Configuration

```rust
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "mcp-file-server")]
#[command(about = "A file management MCP server")]
pub struct Config {
    /// Base directory for file operations
    #[arg(short, long, default_value = "./data")]
    pub base_dir: PathBuf,

    /// Maximum file size in bytes
    #[arg(long, default_value = "10485760")] // 10MB
    pub max_file_size: u64,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}
```

### 2. Implement File Management Tools

```rust
use mocopr_server::handlers::ToolHandler;
use mocopr_core::prelude::*;
use serde_json::{json, Value};
use std::path::PathBuf;
use async_trait::async_trait;

pub struct ListFilesTool {
    base_dir: PathBuf,
}

impl ListFilesTool {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    async fn execute_impl(&self, params: Value) -> anyhow::Result<Value> {
        let relative_path = params.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        // Validate path to prevent directory traversal
        let full_path = self.base_dir.join(relative_path);
        if !full_path.starts_with(&self.base_dir) {
            return Err(anyhow::anyhow!("Path traversal not allowed"));
        }

        let entries = tokio::fs::read_dir(full_path).await?;
        let mut files = Vec::new();
        let mut dirs = Vec::new();

        let mut entries = entries;
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            let name = entry.file_name().to_string_lossy().to_string();

            if metadata.is_dir() {
                dirs.push(json!({
                    "name": name,
                    "type": "directory"
                }));
            } else {
                files.push(json!({
                    "name": name,
                    "type": "file",
                    "size": metadata.len()
                }));
            }
        }

        Ok(json!({
            "directories": dirs,
            "files": files
        }))
    }
}

#[async_trait]
impl ToolHandler for ListFilesTool {
    async fn tool(&self) -> Tool {
        Tool::new(
            "list_files",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the directory to list (optional, defaults to current directory)"
                    }
                }
            })
        ).with_description("List files and directories in a given path")
    }

    async fn call(&self, arguments: Option<serde_json::Value>) -> Result<ToolsCallResponse> {
        let args = arguments.unwrap_or_default();
        match self.execute_impl(args).await {
            Ok(result) => Ok(ToolsCallResponse::success(vec![
                Content::Text(TextContent::new(result.to_string())),
            ])),
            Err(e) => Ok(ToolsCallResponse::error(vec![
                Content::Text(TextContent::new(e.to_string())),
            ])),
        }
    }
}

```rust
pub struct ReadFileTool {
    base_dir: PathBuf,
}

impl ReadFileTool {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    async fn execute_impl(&self, params: Value) -> anyhow::Result<Value> {
        let file_path = params["file_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing file_path parameter"))?;

        let full_path = self.base_dir.join(file_path);
        if !full_path.starts_with(&self.base_dir) {
            return Err(anyhow::anyhow!("Path traversal not allowed"));
        }

        let content = tokio::fs::read_to_string(&full_path).await
            .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

        Ok(json!({
            "content": content,
            "path": file_path
        }))
    }
}

#[async_trait]
impl ToolHandler for ReadFileTool {
    async fn tool(&self) -> Tool {
        Tool::new(
            "read_file",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Relative path to the file to read"
                    }
                },
                "required": ["file_path"]
            })
        ).with_description("Read the contents of a file")
    }

    async fn call(&self, arguments: Option<serde_json::Value>) -> Result<ToolsCallResponse> {
        let args = arguments.ok_or_else(|| Error::InvalidRequest("Missing arguments".to_string()))?;
        match self.execute_impl(args).await {
            Ok(result) => Ok(ToolsCallResponse::success(vec![
                Content::Text(TextContent::new(result.to_string())),
            ])),
            Err(e) => Ok(ToolsCallResponse::error(vec![
                Content::Text(TextContent::new(e.to_string())),
            ])),
        }
    }
}

```rust
pub struct WriteFileTool {
    base_dir: PathBuf,
    max_file_size: u64,
}

impl WriteFileTool {
    pub fn new(base_dir: PathBuf, max_file_size: u64) -> Self {
        Self { base_dir, max_file_size }
    }

    async fn execute_impl(&self, params: Value) -> anyhow::Result<Value> {
        let file_path = params["file_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing file_path parameter"))?;

        let content = params["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing content parameter"))?;

        // Check file size limit
        if content.len() as u64 > self.max_file_size {
            return Err(anyhow::anyhow!(
                "Content too large. Maximum size: {} bytes",
                self.max_file_size
            ));
        }

        let full_path = self.base_dir.join(file_path);
        if !full_path.starts_with(&self.base_dir) {
            return Err(anyhow::anyhow!("Path traversal not allowed"));
        }

        // Create parent directories if they don't exist
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&full_path, content).await
            .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;

        Ok(json!({
            "success": true,
            "path": file_path,
            "size": content.len()
        }))
    }
}

#[async_trait]
impl ToolHandler for WriteFileTool {
    async fn tool(&self) -> Tool {
        Tool::new(
            "write_file",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Relative path where to write the file"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    }
                },
                "required": ["file_path", "content"]
            })
        ).with_description("Write content to a file")
    }

    async fn call(&self, arguments: Option<serde_json::Value>) -> Result<ToolsCallResponse> {
        let args = arguments.ok_or_else(|| Error::InvalidRequest("Missing arguments".to_string()))?;
        match self.execute_impl(args).await {
            Ok(result) => Ok(ToolsCallResponse::success(vec![
                Content::Text(TextContent::new(result.to_string())),
            ])),
            Err(e) => Ok(ToolsCallResponse::error(vec![
                Content::Text(TextContent::new(e.to_string())),
            ])),
        }
    }
}
```

### 3. Add File Resources

```rust
use mocopr_server::handlers::ResourceHandler;

pub struct FileResource {
    base_dir: PathBuf,
}

impl FileResource {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }
}

#[async_trait]
impl ResourceHandler for FileResource {
    async fn resource(&self) -> Resource {
        Resource {
            uri: url::Url::parse("file:///").unwrap(),
            name: "file".to_string(),
            description: Some("File system resource".to_string()),
            mime_type: None,
            annotations: None,
        }
    }

    async fn read(&self) -> Result<Vec<ResourceContent>> {
        // This is a simple example - in practice you'd implement actual file reading logic
        let content = vec![Content::Text(TextContent::new("Example file content"))];
        let resource_content = ResourceContent {
            uri: url::Url::parse("file:///example.txt").unwrap(),
            mime_type: Some("text/plain".to_string()),
            contents: content,
        };

        Ok(vec![resource_content])
    }
}
```

### 4. Main Server Implementation

```rust
use mocopr_server::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::parse();

    // Initialize logging
    let log_level = if config.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("mcp_file_server={},mocopr={}", log_level, log_level))
        .init();

    tracing::info!("Starting MCP File Server");
    tracing::info!("Base directory: {:?}", config.base_dir);
    tracing::info!("Max file size: {} bytes", config.max_file_size);

    // Ensure base directory exists
    tokio::fs::create_dir_all(&config.base_dir).await?;

    // Build the server
    let server = McpServerBuilder::new()
        .with_info("file-server", "1.0.0")
        .with_tools()
        .with_resources()
        .with_tool(ListFilesTool::new(config.base_dir.clone()))
        .with_tool(ReadFileTool::new(config.base_dir.clone()))
        .with_tool(WriteFileTool::new(config.base_dir.clone(), config.max_file_size))
        .with_resource(FileResource::new(config.base_dir.clone()))
        .build()?;

    // Run the server
    tracing::info!("File server is ready and listening on stdio");
    server.run_stdio().await?;

    Ok(())
}
```

## Adding Middleware

> **Note**: The middleware system shown below is conceptual and represents future planned functionality. The current MoCoPr implementation does not yet include middleware support. For now, focus on implementing your business logic directly in the tool and resource handlers.

### 1. Request Logging (Conceptual)

```rust
// This is a conceptual example - middleware is not yet implemented
use tracing::{info, error};

// For now, add logging directly to your tools:
impl ToolHandler for ListFilesTool {
    async fn call(&self, arguments: Option<serde_json::Value>) -> Result<ToolsCallResponse> {
        let start = std::time::Instant::now();
        tracing::info!("Processing list_files request");

        let result = self.execute_impl(arguments.unwrap_or_default()).await;
        let duration = start.elapsed();

        match &result {
            Ok(_) => tracing::info!("list_files completed in {:?}", duration),
            Err(e) => tracing::error!("list_files failed in {:?}: {}", duration, e),
        }

        // Convert result to ToolsCallResponse...
        result
    }
}
```

### 2. Rate Limiting (Conceptual)

For now, implement rate limiting at the application level or use a reverse proxy like nginx for production deployments.

## Error Handling Best Practices

### 1. Custom Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FileServerError {
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Permission denied: {action}")]
    PermissionDenied { action: String },

    #[error("File too large: {size} bytes (max: {max_size} bytes)")]
    FileTooLarge { size: u64, max_size: u64 },

    #[error("Invalid path: {path}")]
    InvalidPath { path: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Convert to JSON-RPC error
impl From<FileServerError> for JsonRpcError {
    fn from(err: FileServerError) -> Self {
        match err {
            FileServerError::FileNotFound { .. } => JsonRpcError {
                code: -32404,
                message: err.to_string(),
                data: None,
            },
            FileServerError::PermissionDenied { .. } => JsonRpcError {
                code: -32403,
                message: "Access denied".to_string(), // Don't leak details
                data: None,
            },
            FileServerError::FileTooLarge { .. } => JsonRpcError {
                code: -32413,
                message: err.to_string(),
                data: None,
            },
            _ => JsonRpcError {
                code: -32500,
                message: "Internal server error".to_string(),
                data: None,
            },
        }
    }
}
```

## Testing Your Server

### 1. Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_list_files_tool() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let tool = ListFilesTool::new(temp_dir.path().to_path_buf());

        // Create test file
        tokio::fs::write(temp_dir.path().join("test.txt"), "content").await?;

        let response = tool.call(Some(json!({}))).await?;
        // Extract the actual result from the ToolsCallResponse
        if let Some(Content::Text(text_content)) = response.contents.first() {
            let result: serde_json::Value = serde_json::from_str(&text_content.text)?;
            let files = &result["files"];
            assert!(files.is_array());
            assert_eq!(files.as_array().unwrap().len(), 1);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_read_write_file() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let write_tool = WriteFileTool::new(temp_dir.path().to_path_buf(), 1024);
        let read_tool = ReadFileTool::new(temp_dir.path().to_path_buf());

        let content = "Hello, MCP World!";
        let write_response = write_tool.call(Some(json!({
            "file_path": "hello.txt",
            "content": content
        }))).await?;

        // Verify write succeeded by checking the response contents
        assert!(write_response.contents.len() > 0);

        let read_response = read_tool.call(Some(json!({
            "file_path": "hello.txt"
        }))).await?;

        // Extract and verify the content
        if let Some(Content::Text(text_content)) = read_response.contents.first() {
            let result: serde_json::Value = serde_json::from_str(&text_content.text)?;
            assert_eq!(result["content"], content);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_path_traversal_protection() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let tool = ReadFileTool::new(temp_dir.path().to_path_buf());

        let response = tool.call(Some(json!({
            "file_path": "../../../etc/passwd"
        }))).await?;

        // Check if the response indicates an error (should be in the contents)
        if let Some(Content::Text(text_content)) = response.contents.first() {
            assert!(text_content.text.contains("Path traversal"));
        }

        Ok(())
    }
}
```

### 2. Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use mocopr_client::Client;

    #[tokio::test]
    async fn test_full_server_client_interaction() -> anyhow::Result<()> {
        // This would require more complex setup with actual server/client communication
        // For now, we'll test the server building process

        let temp_dir = TempDir::new()?;

        let server = McpServerBuilder::new()
            .with_info("test-server", "1.0.0")
            .with_tools()
            .with_tool(ListFilesTool::new(temp_dir.path().to_path_buf()))
            .build()?;

        // Verify server was built successfully
        assert!(server.capabilities().tools.len() > 0);

        Ok(())
    }
}
```

## Running and Testing

### 1. Build and run your server

```bash
cargo run -- --base-dir ./test-data --verbose
```

### 2. Test with a simple client

Create `test_client.py`:

```python
#!/usr/bin/env python3
import json
import subprocess
import sys

def test_file_server():
    # Start the server process
    server = subprocess.Popen(
        ["cargo", "run", "--", "--base-dir", "./test-data"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )

    # Send initialization request
    init_request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    }

    server.stdin.write(json.dumps(init_request) + "\n")
    server.stdin.flush()

    # Read response
    response = server.stdout.readline()
    print("Init response:", response)

    # Test list_files tool
    list_request = {
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "list_files",
            "arguments": {}
        }
    }

    server.stdin.write(json.dumps(list_request) + "\n")
    server.stdin.flush()

    response = server.stdout.readline()
    print("List files response:", response)

    server.terminate()

if __name__ == "__main__":
    test_file_server()
```

## Next Steps

You now have a fully functional MCP file server! Next, explore:

- [Advanced Features](03-advanced-features.md) - Add prompts, advanced middleware
- [Production Deployment](04-production-deployment.md) - Deploy with Docker, monitoring
- [Performance Tuning](05-performance-tuning.md) - Optimize for high throughput

## Common Patterns

### Validation Middleware

Always validate inputs at the middleware level:

```rust
#[derive(Clone)]
pub struct ValidationMiddleware;

#[async_trait::async_trait]
impl Middleware for ValidationMiddleware {
    async fn handle(&self, mut request: JsonRpcRequest, next: Next) -> Result<JsonRpcResponse> {
        // Validate JSON-RPC format
        if request.jsonrpc != "2.0" {
            return Err(anyhow::anyhow!("Invalid JSON-RPC version"));
        }

        // Validate method names
        if !request.method.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '/') {
            return Err(anyhow::anyhow!("Invalid method name"));
        }

        next.run(request).await
    }
}
```

### Async Resource Cleanup

Always implement proper cleanup:

```rust
impl Drop for FileServerState {
    fn drop(&mut self) {
        // Cleanup resources
        tracing::info!("Cleaning up file server resources");
    }
}
```
