//! Example demonstrating RBAC integration with MoCoPr
//!
//! This example shows how to set up a secure MCP server with role-based access control
//! using the role-system crate integration.

use async_trait::async_trait;
use mocopr_core::prelude::*;
use mocopr_rbac::context::ContextConditions;
use mocopr_rbac::prelude::*;
use mocopr_server::handlers::ToolHandler;
use mocopr_server::prelude::*;
use smallvec::SmallVec;
use std::future::Future;
use std::pin::Pin;
use tracing::{info, Level};

// Type alias to simplify the complex function signature
type ToolHandlerFn = Box<
    dyn Fn(
            serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = mocopr_core::Result<serde_json::Value>> + Send>>
        + Send
        + Sync,
>;

#[tokio::main]
async fn main() -> mocopr_core::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting RBAC-enabled MCP server example");

    // Create RBAC middleware with custom configuration
    let _rbac = RbacMiddleware::builder()
        .with_default_roles() // Creates: guest, user, power_user, admin
        .with_audit_logging(true)
        // Add custom roles
        .with_role(
            "calculator_user",
            &[
                "list:tools",
                "call:tools:calculator/*",
                "read:resources:public/*",
            ],
        )
        .with_role(
            "file_admin",
            &[
                "list:tools",
                "call:tools:file/*",
                "read:resources:*",
                "list:resources",
            ],
        )
        // Add conditional permissions - admin tools only during business hours
        .with_conditional_permission(
            "power_user",
            "call:tools:admin/*",
            ContextConditions::business_hours_only(),
        )
        // Sensitive operations only for high-trust clients
        .with_conditional_permission("admin", "call:tools:dangerous/*", |context| {
            context.get("trust_level") == Some(&"high".to_string())
                && context.get("user_id").is_some()
        })
        .build()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    // Create sample tools with different security levels
    let calculator_tool = create_calculator_tool();
    let file_tool = create_file_tool();
    let admin_tool = create_admin_tool();
    let dangerous_tool = create_dangerous_tool();

    // Build the MCP server with RBAC middleware
    let server = McpServerBuilder::new()
        .with_info("RBAC Example Server", "1.0.0")
        .with_tool(calculator_tool)
        .with_tool(file_tool)
        .with_tool(admin_tool)
        .with_tool(dangerous_tool)
        .build()?;

    info!("Server built with RBAC enabled");
    info!("Available roles:");
    info!("  - guest: minimal access (list tools/resources)");
    info!("  - user: standard access (call tools, read resources)");
    info!("  - power_user: advanced access + conditional admin tools");
    info!("  - admin: full access + conditional dangerous tools");
    info!("  - calculator_user: custom role for calculator access");
    info!("  - file_admin: custom role for file operations");
    info!("");
    info!("To test different access levels, send requests with auth parameters:");
    info!("  {{\"auth\": {{\"subject_id\": \"user1\", \"subject_type\": \"user\"}}}}");
    info!("");
    info!("For conditional permissions, add context:");
    info!("  {{\"context\": {{\"trust_level\": \"high\"}}}}");

    // Run the server
    server.run_stdio().await?;

    Ok(())
}

/// Create a calculator tool (safe for most users)
fn create_calculator_tool() -> impl ToolHandler {
    SimpleTool::new(
        "calculator/add".to_string(),
        "Add two numbers".to_string(),
        Box::new(|args| {
            Box::pin(async move {
                let a: f64 = args
                    .get("a")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| Error::InvalidParams("Missing parameter 'a'".to_string()))?;
                let b: f64 = args
                    .get("b")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| Error::InvalidParams("Missing parameter 'b'".to_string()))?;

                Ok(serde_json::json!({
                    "result": a + b
                }))
            })
        }),
    )
}

/// Create a file tool (requires file permissions)
fn create_file_tool() -> impl ToolHandler {
    SimpleTool::new(
        "file/list".to_string(),
        "List files in directory".to_string(),
        Box::new(|args| {
            Box::pin(async move {
                let path: String = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or(".")
                    .to_string();

                // Simulate file listing
                Ok(serde_json::json!({
                    "files": ["file1.txt", "file2.txt", "file3.txt"],
                    "path": path
                }))
            })
        }),
    )
}

/// Create an admin tool (requires admin role + business hours)
fn create_admin_tool() -> impl ToolHandler {
    SimpleTool::new(
        "admin/restart".to_string(),
        "Restart the server (admin only)".to_string(),
        Box::new(|_args| {
            Box::pin(async move {
                info!("Admin tool called - server restart requested");
                Ok(serde_json::json!({
                    "status": "restart_scheduled",
                    "message": "Server will restart in 30 seconds"
                }))
            })
        }),
    )
}

/// Create a dangerous tool (requires admin role + high trust + user verification)
fn create_dangerous_tool() -> impl ToolHandler {
    SimpleTool::new(
        "dangerous/delete_all".to_string(),
        "Delete all data (extremely dangerous!)".to_string(),
        Box::new(|_args| {
            Box::pin(async move {
                info!("DANGEROUS tool called - this would delete all data!");
                Ok(serde_json::json!({
                    "status": "simulated",
                    "message": "This is a simulation - no data was actually deleted"
                }))
            })
        }),
    )
}

/// Simple tool implementation for the example
struct SimpleTool {
    name: String,
    description: String,
    handler: ToolHandlerFn,
}

impl SimpleTool {
    fn new(name: String, description: String, handler: ToolHandlerFn) -> Self {
        Self {
            name,
            description,
            handler,
        }
    }
}

#[async_trait]
impl ToolHandler for SimpleTool {
    async fn tool(&self) -> Tool {
        Tool {
            name: self.name.clone(),
            description: Some(self.description.clone()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "a": {"type": "number", "description": "First number"},
                    "b": {"type": "number", "description": "Second number"}
                },
                "required": ["a", "b"]
            }),
        }
    }

    async fn call(
        &self,
        arguments: Option<serde_json::Value>,
    ) -> mocopr_core::Result<ToolsCallResponse> {
        let args = arguments.unwrap_or(serde_json::json!({}));
        let result = (self.handler)(args).await?;

        let content = SmallVec::from_vec(vec![Content::Text(TextContent::new(result.to_string()))]);

        Ok(ToolsCallResponse {
            content,
            is_error: Some(false),
            meta: ResponseMetadata::new(),
        })
    }
}
