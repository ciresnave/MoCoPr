//! Integration tests for MCP tools functionality

use mocopr_core::Result;
use mocopr_core::types::tools::{Tool, ToolsCallResponse};
use mocopr_server::handlers::ToolHandler;
use serde_json::json;
use smallvec::SmallVec;

// Simple test tool implementing ToolHandler
struct TestTool;

#[async_trait::async_trait]
impl ToolHandler for TestTool {
    async fn tool(&self) -> Tool {
        Tool {
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                },
                "required": ["name"]
            }),
        }
    }

    async fn call(&self, arguments: Option<serde_json::Value>) -> Result<ToolsCallResponse> {
        let args = arguments.unwrap_or_default();
        let name = args["name"].as_str().unwrap_or("World");

        let mut content = SmallVec::new();
        content.push(mocopr_core::types::Content::Text(
            mocopr_core::types::TextContent {
                text: format!("Hello, {name}!"),
                annotations: None,
            },
        ));

        Ok(ToolsCallResponse {
            content,
            is_error: None,
            meta: Default::default(),
        })
    }
}

#[tokio::test]
async fn test_tool_creation() -> anyhow::Result<()> {
    let tool = TestTool;

    // Test tool info
    let tool_info = tool.tool().await;
    assert_eq!(tool_info.name, "test_tool");
    assert_eq!(tool_info.description, Some("A test tool".to_string()));

    // Test tool execution
    let args = json!({"name": "World"});
    let response = tool.call(Some(args)).await?;

    // Check response
    assert!(!response.content.is_empty());
    if let Some(content) = response.content.first()
        && let mocopr_core::types::Content::Text(text_content) = content
    {
        assert!(text_content.text.contains("Hello, World!"));
    }

    Ok(())
}
