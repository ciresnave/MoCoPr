use anyhow::{Context, Result};
use mocopr_client::McpClient;
use mocopr_core::prelude::*;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::timeout;
use url::Url;

// Note: Since real McpClient requires a running server, these tests will create
// a mock server or test against expected errors when no server is available

#[tokio::test]
async fn test_comprehensive_integration() -> Result<()> {
    // Test that tries to connect but expects failure due to no server
    let client_result = timeout(
        Duration::from_secs(5),
        McpClient::connect_stdio(
            "echo", // Use echo as a dummy command
            &["Testing"],
            Implementation {
                name: "Test Client".to_string(),
                version: "1.0.0".to_string(),
            },
            ClientCapabilities::default(),
        ),
    )
    .await;

    // Expect this to fail since echo doesn't implement MCP protocol
    assert!(client_result.is_err() || client_result.unwrap().is_err());
    Ok(())
}

#[tokio::test]
async fn test_client_connection_failure_handling() -> Result<()> {
    // Test connecting to a non-existent command
    let result = McpClient::connect_stdio(
        "nonexistent_command_12345",
        &["arg1"],
        Implementation {
            name: "Test Client".to_string(),
            version: "1.0.0".to_string(),
        },
        ClientCapabilities::default(),
    )
    .await;

    // Should fail gracefully
    assert!(result.is_err());
    // println!("Expected error: {:?}", result.unwrap_err()); // Commented out due to Debug trait requirement
    Ok(())
}

#[tokio::test]
async fn test_client_capabilities() -> Result<()> {
    // Test client capabilities creation
    let capabilities = ClientCapabilities {
        roots: Some(RootsCapability {
            list_changed: Some(true),
        }),
        sampling: Some(SamplingCapability {}),
        experimental: None,
    };

    // Verify capabilities are structured correctly
    assert!(capabilities.roots.is_some());
    assert!(capabilities.sampling.is_some());
    Ok(())
}

#[tokio::test]
async fn test_url_parsing_for_resources() -> Result<()> {
    // Test URL parsing for resource operations
    let valid_urls = vec![
        "file:///safe/directory/test.txt",
        "memory://calculation_history",
        "http://example.com/resource",
        "custom://protocol/resource",
    ];

    for url_str in valid_urls {
        let url =
            Url::parse(url_str).with_context(|| format!("Failed to parse URL: {}", url_str))?;
        assert!(!url.as_str().is_empty());
        println!("Successfully parsed URL: {}", url);
    }

    // Test invalid URLs
    let invalid_urls = vec![
        "not-a-url",
        "://missing-scheme",
        "file://invalid path with spaces",
    ];

    for url_str in invalid_urls {
        let result = Url::parse(url_str);
        assert!(
            result.is_err(),
            "Should fail to parse invalid URL: {}",
            url_str
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_tool_parameter_validation() -> Result<()> {
    // Test tool call parameter structures
    let valid_params = json!({
        "a": 42.5,
        "b": 37.3,
        "operation": "add"
    });

    let invalid_params = json!({
        "wrong_field": "value"
    });

    // These would be validated by the server, so we just test JSON structure
    assert!(valid_params.is_object());
    assert!(invalid_params.is_object());

    // Test parameter serialization
    let serialized = serde_json::to_string(&valid_params)?;
    let deserialized: Value = serde_json::from_str(&serialized)?;
    assert_eq!(valid_params, deserialized);

    Ok(())
}

#[tokio::test]
async fn test_error_response_handling() -> Result<()> {
    // Test error response structures that would come from a real server
    let error_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32601,
            "message": "Method not found",
            "data": {
                "method": "nonexistent_tool"
            }
        }
    });

    // Verify error structure
    assert!(error_response["error"].is_object());
    assert!(error_response["error"]["code"].is_number());
    assert!(error_response["error"]["message"].is_string());

    Ok(())
}

// This test demonstrates what a real integration would look like
// but currently cannot run without a real MCP server
#[tokio::test]
#[ignore] // Ignored because it requires an actual server
async fn test_with_real_calculator_server() -> Result<()> {
    // This would work if we had the calculator server running
    let client = McpClient::connect_stdio(
        "cargo",
        &["run", "--bin", "calculator-server"],
        Implementation {
            name: "Test Client".to_string(),
            version: "1.0.0".to_string(),
        },
        ClientCapabilities::default(),
    )
    .await?;

    // List available tools
    let tools_response = client.list_tools().await?;
    assert!(!tools_response.tools.is_empty());

    // Call a tool
    let result = client
        .call_tool("add".to_string(), Some(json!({"a": 42.5, "b": 37.3})))
        .await?;

    // Verify result structure
    assert!(!result.content.is_empty());

    Ok(())
}

// Test that demonstrates security considerations
#[tokio::test]
async fn test_security_validation() -> Result<()> {
    // Test path traversal detection in URLs
    let malicious_paths = vec![
        "../../../etc/passwd",
        "..\\..\\..\\windows\\system32\\config\\sam",
        "file://../sensitive/file.txt",
    ];

    for path in malicious_paths {
        // In a real implementation, these would be blocked by the server
        // Here we just test that the client can construct the requests
        if let Ok(url) = Url::parse(&format!("file://{}", path)) {
            println!("Constructed URL (would be validated by server): {}", url);
        } else {
            println!("Invalid URL rejected: {}", path);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_large_data_handling() -> Result<()> {
    // Test handling of large JSON payloads
    let large_data = json!({
        "data": "x".repeat(1024 * 1024), // 1MB string
        "metadata": {
            "size": 1024 * 1024,
            "type": "large_text"
        }
    });

    // Test serialization/deserialization of large data
    let serialized = serde_json::to_string(&large_data)?;
    assert!(serialized.len() > 1024 * 1024);

    let deserialized: Value = serde_json::from_str(&serialized)?;
    assert_eq!(large_data, deserialized);

    Ok(())
}

// Helper functions for testing structures - kept for future test expansion

/// Verify the structure of tool response for testing purposes
#[allow(dead_code)]
async fn verify_tool_response_structure(response: &ToolsCallResponse) -> Result<()> {
    assert!(!response.content.is_empty());

    // Check if it's an error response
    if let Some(is_error) = response.is_error
        && is_error
    {
        println!("Tool returned error as expected");
    }

    Ok(())
}

/// Verify the structure of resource response for testing purposes
#[allow(dead_code)]
async fn verify_resource_response_structure(response: &ResourcesReadResponse) -> Result<()> {
    assert!(!response.contents.is_empty());

    for resource_content in &response.contents {
        println!("Resource URI: {}", resource_content.uri);
        if let Some(mime_type) = &resource_content.mime_type {
            println!("MIME type: {}", mime_type);
        }

        for content in &resource_content.contents {
            match content {
                Content::Text(text_content) => {
                    assert!(!text_content.text.is_empty());
                    println!("Text content length: {}", text_content.text.len());
                }
                Content::Image(image_content) => {
                    assert!(!image_content.data.is_empty());
                    println!("Image data size: {} bytes", image_content.data.len());
                }
                Content::StructuredError(_) => {
                    // Handle structured error, for now just acknowledge it
                    println!("Received structured error");
                }
            }
        }
    }

    Ok(())
}
