#![allow(dead_code)]

use anyhow::Result;
use mocopr_core::utils::Utils;
use mocopr_core::{PromptGenerator, ResourceReader, ToolExecutor};
use mocopr_macros::{Prompt, Resource, Tool};
use mocopr_server::prelude::*;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

/// Maximum file size allowed for reading (10MB)
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// A file system resource that can read files from a specified directory
#[derive(Resource)]
#[resource(name = "file", description = "Read files from the file system")]
struct FileResource {
    /// The root directory to serve files from
    root_dir: String,
}

impl FileResource {
    fn new(root_dir: String) -> Self {
        Self { root_dir }
    }

    async fn read_file(&self, path: &str) -> Result<String> {
        // Validate the path input for safety
        Utils::validate_safe_string(path)?;

        // Sanitize the path to prevent directory traversal
        let sanitized_path = Utils::sanitize_path(path);
        let full_path = Path::new(&self.root_dir).join(&sanitized_path);

        // Resolve canonical paths for security comparison
        let canonical_root = fs::canonicalize(&self.root_dir)?;
        let canonical_path = fs::canonicalize(&full_path)?;

        if !canonical_path.starts_with(&canonical_root) {
            anyhow::bail!("Path is outside of allowed directory: access denied");
        }

        // Check file size before reading
        let metadata = fs::metadata(&canonical_path)?;
        Utils::validate_file_size(metadata.len(), MAX_FILE_SIZE)?;

        let content = fs::read_to_string(&canonical_path)?;

        info!(
            "File read successfully: {} ({} bytes)",
            canonical_path.display(),
            Utils::format_bytes(content.len() as u64)
        );

        Ok(content)
    }
}

#[async_trait::async_trait]
impl ResourceReader for FileResource {
    async fn read_resource(&self) -> mocopr_core::Result<Vec<ResourceContent>> {
        // For this example, let's list the files in the root directory as the resource
        let entries: Result<Vec<_>, _> = fs::read_dir(&self.root_dir)?.collect();
        let entries = entries.map_err(|e| mocopr_core::Error::Internal(e.to_string()))?;

        let mut files = Vec::new();
        for entry in entries {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if entry
                .file_type()
                .map_err(|e| mocopr_core::Error::Internal(e.to_string()))?
                .is_file()
            {
                let metadata = entry
                    .metadata()
                    .map_err(|e| mocopr_core::Error::Internal(e.to_string()))?;
                files.push(json!({
                    "name": file_name,
                    "size": metadata.len(),
                    "size_formatted": Utils::format_bytes(metadata.len())
                }));
            }
        }

        let uri = url::Url::parse("resource://file_listing")
            .map_err(|e| mocopr_core::Error::Internal(e.to_string()))?;
        let content = vec![Content::Text(TextContent::new(
            json!({
                "files": files,
                "root_directory": &self.root_dir
            })
            .to_string(),
        ))];

        Ok(vec![ResourceContent::new(uri, content)])
    }
}

/// A tool to list files in a directory
#[derive(Tool)]
#[tool(
    name = "list_files",
    description = "List files and directories in a given path"
)]
struct ListFilesTool {
    root_dir: String,
}

impl ListFilesTool {
    fn new(root_dir: String) -> Self {
        Self { root_dir }
    }

    async fn execute_impl(&self, args: Value) -> Result<Value> {
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        // Validate and sanitize the path input
        Utils::validate_safe_string(path)?;
        let sanitized_path = Utils::sanitize_path(path);
        let full_path = Path::new(&self.root_dir).join(&sanitized_path);

        // Security check: ensure path is within allowed directory
        let canonical_root = fs::canonicalize(&self.root_dir)?;
        let canonical_path = fs::canonicalize(&full_path)?;

        if !canonical_path.starts_with(&canonical_root) {
            anyhow::bail!("Path is outside of allowed directory: access denied");
        }

        let mut files = Vec::new();
        let mut dirs = Vec::new();

        for entry in fs::read_dir(&canonical_path)? {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Validate file name for safety
            if let Err(e) = Utils::validate_safe_string(&file_name) {
                warn!("Skipping unsafe file name: {} - {}", file_name, e);
                continue;
            }

            if entry.file_type()?.is_dir() {
                dirs.push(file_name);
            } else {
                let metadata = entry.metadata()?;
                files.push(json!({
                    "name": file_name,
                    "size": metadata.len(),
                    "size_formatted": Utils::format_bytes(metadata.len())
                }));
            }
        }

        info!(
            "Listed directory: {} ({} files, {} directories)",
            canonical_path.display(),
            files.len(),
            dirs.len()
        );

        Ok(json!({
            "files": files,
            "directories": dirs,
            "path": path
        }))
    }
}

#[async_trait::async_trait]
impl ToolExecutor for ListFilesTool {
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

/// A tool to search for files by name pattern
#[derive(Tool)]
#[tool(
    name = "search_files",
    description = "Search for files by name pattern"
)]
struct SearchFilesTool {
    root_dir: String,
}

impl SearchFilesTool {
    fn new(root_dir: String) -> Self {
        Self { root_dir }
    }

    async fn execute_impl(&self, args: Value) -> Result<Value> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: pattern"))?;

        let mut results = Vec::new();
        self.search_recursive(Path::new(&self.root_dir), pattern, &mut results)?;

        Ok(json!({
            "matches": results,
            "pattern": pattern
        }))
    }

    fn search_recursive(&self, dir: &Path, pattern: &str, results: &mut Vec<String>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            if file_name.contains(pattern)
                && let Ok(relative_path) = path.strip_prefix(&self.root_dir)
            {
                results.push(relative_path.to_string_lossy().to_string());
            }

            if path.is_dir() {
                self.search_recursive(&path, pattern, results)?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ToolExecutor for SearchFilesTool {
    async fn execute(
        &self,
        args: Option<Value>,
    ) -> mocopr_core::Result<mocopr_core::types::ToolsCallResponse> {
        let args = args.unwrap_or_default();
        match self.execute_impl(args).await {
            Ok(result) => Ok(mocopr_core::types::ToolsCallResponse::success(vec![
                mocopr_core::types::Content::Text(mocopr_core::types::TextContent::new(
                    serde_json::to_string(&result)?,
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

/// A prompt to suggest file operations
#[derive(Prompt)]
#[prompt(
    name = "file_operations",
    description = "Suggest file operations based on current directory state"
)]
struct FileOperationsPrompt {
    root_dir: String,
}

impl FileOperationsPrompt {
    fn new(root_dir: String) -> Self {
        Self { root_dir }
    }

    async fn execute_impl(&self, args: Option<Value>) -> Result<String> {
        let path = args
            .as_ref()
            .and_then(|v| v.get("path"))
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let full_path = Path::new(&self.root_dir).join(path);

        if !full_path.exists() {
            return Ok(format!(
                "Directory '{}' does not exist. Suggested operations:\n\
                              - Create the directory\n\
                              - Check if the path is correct",
                path
            ));
        }

        let entries: Result<Vec<_>, _> = fs::read_dir(&full_path)?.collect();
        let entries = entries?;

        let file_count = entries
            .iter()
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .count();
        let dir_count = entries
            .iter()
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .count();

        let mut suggestions = Vec::<String>::new();

        if file_count == 0 && dir_count == 0 {
            suggestions.push(
                "Directory is empty - consider adding some files or subdirectories".to_string(),
            );
        } else {
            suggestions.push(format!(
                "Found {} files and {} directories",
                file_count, dir_count
            ));

            if file_count > 10 {
                suggestions.push(
                    "Large number of files - consider organizing into subdirectories".to_string(),
                );
            }

            // Check for common file types
            let has_rust = entries
                .iter()
                .any(|e| e.file_name().to_string_lossy().ends_with(".rs"));
            let has_python = entries
                .iter()
                .any(|e| e.file_name().to_string_lossy().ends_with(".py"));
            let has_js = entries
                .iter()
                .any(|e| e.file_name().to_string_lossy().ends_with(".js"));

            if has_rust {
                suggestions.push("Rust files detected - you can use `cargo` commands".to_string());
            }
            if has_python {
                suggestions.push(
                    "Python files detected - consider using virtual environments".to_string(),
                );
            }
            if has_js {
                suggestions.push(
                    "JavaScript files detected - check for package.json or consider npm/yarn"
                        .to_string(),
                );
            }
        }

        Ok(format!(
            "File system analysis for '{}':\n{}",
            path,
            suggestions.join("\n- ")
        ))
    }
}

#[async_trait]
impl PromptGenerator for FileOperationsPrompt {
    async fn generate_prompt(
        &self,
        arguments: Option<HashMap<String, String>>,
    ) -> mocopr_core::Result<mocopr_core::types::PromptsGetResponse> {
        let path = arguments
            .as_ref()
            .and_then(|args| args.get("path"))
            .map(|s| s.as_str())
            .unwrap_or(".");

        let args_value = arguments
            .clone()
            .map(|args| json!(args.into_iter().collect::<HashMap<_, _>>()));

        let response_text = self
            .execute_impl(args_value)
            .await
            .map_err(|e| mocopr_core::Error::Internal(e.to_string()))?;

        let message = mocopr_core::types::PromptMessage::user(response_text);

        Ok(mocopr_core::types::PromptsGetResponse {
            description: Some(format!("File operations suggestions for: {}", path)),
            messages: vec![message],
            meta: mocopr_core::types::ResponseMetadata { _meta: None },
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get the root directory from command line args or use current directory
    let args: Vec<String> = std::env::args().collect();
    let root_dir = args.get(1).unwrap_or(&".".to_string()).clone();

    // Validate that the root directory exists
    if !Path::new(&root_dir).exists() {
        anyhow::bail!("Root directory '{}' does not exist", root_dir);
    }

    info!("Starting MCP File Server with root directory: {}", root_dir);

    // Create resources
    let file_resource = FileResource::new(root_dir.clone());

    // Create tools
    let list_files_tool = ListFilesTool::new(root_dir.clone());
    let search_files_tool = SearchFilesTool::new(root_dir.clone());

    // Create prompts
    let file_operations_prompt = FileOperationsPrompt::new(root_dir.clone());

    // Build and start the server
    let server = McpServerBuilder::new()
        .with_info("File Server", "1.0.0")
        .with_resources()
        .with_tools()
        .with_prompts()
        .with_resource(file_resource)
        .with_tool(list_files_tool)
        .with_tool(search_files_tool)
        .with_prompt(file_operations_prompt)
        .build()?;

    info!("MCP File Server ready. Capabilities:");
    info!("- Resources: file (read files from {}/)", root_dir);
    info!("- Tools: list_files, search_files");
    info!("- Prompts: file_operations");

    // Run the server using stdio transport
    server.run_stdio().await?;

    Ok(())
}
