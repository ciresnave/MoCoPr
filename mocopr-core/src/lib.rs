//! # MoCoPr Core
//!
//! Core types and protocol implementation for the Model Context Protocol (MCP).
//!
//! This crate provides the fundamental building blocks for MCP implementations,
//! including message types, transport abstractions, and protocol utilities.
//!
//! ## API Stability
//!
//! **Current Status: EXPERIMENTAL (v0.x.x)**
//!
//! This library is in active development. APIs marked as:
//! - 🔴 **Experimental**: May change or be removed without notice
//! - 🟡 **Unstable**: May change with deprecation warnings
//! - 🟢 **Stable**: Will follow semantic versioning
//!
//! Most APIs are currently **experimental** until v1.0.0 release.
//!
//! ## Breaking Changes Policy
//!
//! During the 0.x.x series:
//! - Minor version bumps (0.1.x → 0.2.x) may include breaking changes
//! - Patch version bumps (0.1.0 → 0.1.1) will be backward compatible
//! - All breaking changes will be documented in CHANGELOG.md
//!
//! Starting from 1.0.0:
//! - Semantic versioning will be strictly followed
//! - Breaking changes only in major versions (1.x.x → 2.x.x)
//!
//! ## Usage Recommendations
//!
//! For production use:
//! - Pin to exact versions in Cargo.toml: `mocopr-core = "=0.1.0"`
//! - Subscribe to release notifications for breaking changes
//! - Review CHANGELOG.md before updating

#![warn(missing_docs)]

pub mod error;
/// Production monitoring and observability system
pub mod monitoring;
pub mod protocol;
/// Security validation and hardening system
pub mod security;
pub mod transport;
pub mod types;
pub mod utils;

pub use error::{Error, Result};
pub use protocol::*;
pub use transport::{Transport, TransportConfig, TransportFactory};
pub use types::*;

/// Trait that users must implement to provide actual tool functionality
///
/// This trait separates the tool metadata (handled by the derive macro) from
/// the tool execution logic (implemented by the user). It's designed to work
/// with the `#[derive(Tool)]` macro from `mocopr_macros`.
///
/// # Example
///
/// ```rust
/// use mocopr_core::{ToolExecutor, types::{ToolsCallResponse, Content, TextContent}, Result};
/// use serde_json::Value;
/// use async_trait::async_trait;
///
/// struct Calculator;
///
/// #[async_trait]
/// impl ToolExecutor for Calculator {
///     async fn execute(&self, arguments: Option<Value>) -> Result<ToolsCallResponse> {
///         let args = arguments.unwrap_or_default();
///         let operation = args.get("operation").and_then(|v| v.as_str()).unwrap_or("add");
///         let a: f64 = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
///         let b: f64 = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
///
///         let result = match operation {
///             "add" => a + b,
///             "subtract" => a - b,
///             "multiply" => a * b,
///             "divide" => {
///                 if b == 0.0 {
///                     return Ok(ToolsCallResponse::error(vec![
///                         Content::Text(TextContent::new("Division by zero"))
///                     ]));
///                 }
///                 a / b
///             }
///             _ => return Ok(ToolsCallResponse::error(vec![
///                 Content::Text(TextContent::new("Unknown operation"))
///             ])),
///         };
///
///         Ok(ToolsCallResponse::success(vec![
///             Content::Text(TextContent::new(&result.to_string()))
///         ]))
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait ToolExecutor {
    /// Execute the tool with the given arguments.
    ///
    /// This method must be implemented by the user to provide the actual
    /// tool functionality. The arguments parameter contains the JSON
    /// arguments passed to the tool.
    ///
    /// # Arguments
    ///
    /// * `arguments` - Optional JSON value containing tool arguments
    ///
    /// # Returns
    ///
    /// A `ToolsCallResponse` containing the tool execution results
    async fn execute(
        &self,
        arguments: Option<serde_json::Value>,
    ) -> Result<types::ToolsCallResponse>;

    /// Provide the JSON schema for tool arguments (optional)
    ///
    /// Override this method to provide a custom JSON schema for tool arguments.
    /// If not overridden, a basic empty object schema is used.
    ///
    /// # Returns
    ///
    /// Optional JSON schema for the tool's arguments
    async fn schema(&self) -> Option<serde_json::Value> {
        None
    }
}

/// Trait that users must implement to provide actual resource functionality
///
/// This trait separates the resource metadata (handled by the derive macro) from
/// the resource reading logic (implemented by the user). It's designed to work
/// with the `#[derive(Resource)]` macro from `mocopr_macros`.
///
/// # Example
///
/// ```rust
/// use mocopr_core::{ResourceReader, types::{ResourceContent, Content, TextContent}, Result};
/// use async_trait::async_trait;
/// use url::Url;
///
/// struct FileResource {
///     path: String,
/// }
///
/// #[async_trait]
/// impl ResourceReader for FileResource {
///     async fn read_resource(&self) -> Result<Vec<ResourceContent>> {
///         // Read file content (simplified example)
///         let content = std::fs::read_to_string(&self.path)
///             .map_err(|e| mocopr_core::Error::resource_error(format!("Failed to read file: {}", e)))?;
///
///         let uri = Url::parse(&format!("file://{}", self.path))
///             .map_err(|e| mocopr_core::Error::validation(format!("Invalid URI: {}", e)))?;
///
///         Ok(vec![ResourceContent::new(uri, vec![
///             Content::Text(TextContent::new(&content))
///         ])])
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait ResourceReader {
    /// Read the resource content.
    ///
    /// This method must be implemented by the user to provide the actual
    /// resource reading functionality.
    ///
    /// # Returns
    ///
    /// A vector of `ResourceContent` objects containing the resource data
    async fn read_resource(&self) -> Result<Vec<types::ResourceContent>>;
}

/// Trait that users must implement to provide actual prompt functionality
///
/// This trait separates the prompt metadata (handled by the derive macro) from
/// the prompt generation logic (implemented by the user). It's designed to work
/// with the `#[derive(Prompt)]` macro from `mocopr_macros`.
///
/// # Example
///
/// ```rust
/// use mocopr_core::{PromptGenerator, types::{PromptsGetResponse, PromptMessage, Role, Content, TextContent, ResponseMetadata}, Result};
/// use std::collections::HashMap;
/// use async_trait::async_trait;
///
/// struct SummaryPrompt;
///
/// #[async_trait]
/// impl PromptGenerator for SummaryPrompt {
///     async fn generate_prompt(&self, arguments: Option<HashMap<String, String>>) -> Result<PromptsGetResponse> {
///         let args = arguments.unwrap_or_default();
///         let text = args.get("text").cloned().unwrap_or_default();
///
///         if text.is_empty() {
///             return Err(mocopr_core::Error::invalid_params("Text parameter is required".to_string()));
///         }
///
///         Ok(PromptsGetResponse {
///             description: Some("Generate a summary of the provided text".to_string()),
///             messages: vec![
///                 PromptMessage {
///                     role: Role::User,
///                     content: Content::Text(TextContent::new(&format!(
///                         "Please summarize the following text:\n\n{}",
///                         text
///                     ))),
///                 }
///             ],
///             meta: ResponseMetadata::default(),
///         })
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait PromptGenerator {
    /// Generate a prompt response with the given arguments.
    ///
    /// This method must be implemented by the user to provide the actual
    /// prompt generation functionality.
    ///
    /// # Arguments
    ///
    /// * `arguments` - Optional key-value pairs for prompt parameters
    ///
    /// # Returns
    ///
    /// A `PromptsGetResponse` containing the generated prompt messages
    async fn generate_prompt(
        &self,
        arguments: Option<std::collections::HashMap<String, String>>,
    ) -> Result<types::PromptsGetResponse>;
}

/// Re-export commonly used types
pub mod prelude {
    pub use crate::PromptGenerator;
    pub use crate::ResourceReader;
    pub use crate::ToolExecutor;
    pub use crate::error::{Error, Result};
    pub use crate::monitoring::{HealthCheck, HealthStatus, MonitoringSystem, PerformanceMetrics};
    pub use crate::protocol::*;
    pub use crate::security::{ErrorRecoverySystem, SecurityValidator};
    pub use crate::transport::{Transport, TransportConfig, TransportFactory};
    pub use crate::types::*;
    pub use crate::utils::Utils;
}
