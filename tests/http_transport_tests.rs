//! Integration tests for HTTP transport
//!
//! These tests verify that the HTTP transport layer works correctly for MCP communication,
//! focusing on the one-way request-response pattern supported by HTTP.

use anyhow::Result;
use mocopr_core::transport::{Transport, http::HttpTransport};
use tokio::time::{Duration, timeout};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// Start a mock HTTP server for testing
async fn start_mock_server() -> MockServer {
    let mock_server = MockServer::start().await;

    // Setup mock response for connectivity test (GET request)
    Mock::given(method("GET"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok",
            "service": "mcp"
        })))
        .mount(&mock_server)
        .await;

    // Setup mock response for MCP messages (POST request)
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "result": "success",
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    mock_server
}

#[tokio::test]
async fn test_http_transport_creation() -> Result<()> {
    let mock_server = start_mock_server().await;
    let endpoint = format!("{}/mcp", mock_server.uri());

    // Create HTTP transport
    let transport = HttpTransport::new(&endpoint).await?;

    // Verify properties
    assert!(transport.is_connected());
    assert_eq!(transport.endpoint(), &endpoint);
    assert_eq!(transport.transport_type(), "http");

    Ok(())
}

#[tokio::test]
async fn test_http_transport_send() -> Result<()> {
    let mock_server = start_mock_server().await;
    let endpoint = format!("{}/mcp", mock_server.uri());

    // Create HTTP transport
    let mut transport = HttpTransport::new(&endpoint).await?;

    // Send a message
    let message = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
    transport.send(message).await?;

    // Check statistics
    let stats = transport.stats().await;
    assert_eq!(stats.messages_sent, 1);
    assert!(stats.bytes_sent > 0);
    assert!(stats.last_activity.is_some());

    Ok(())
}

#[tokio::test]
async fn test_http_transport_receive() -> Result<()> {
    let mock_server = start_mock_server().await;
    let endpoint = format!("{}/mcp", mock_server.uri());

    // Create HTTP transport
    let mut transport = HttpTransport::new(&endpoint).await?;

    // HTTP transport doesn't support receiving, should return None
    let result = timeout(Duration::from_secs(1), transport.receive()).await??;
    assert!(result.is_none());

    Ok(())
}

#[tokio::test]
async fn test_http_transport_close() -> Result<()> {
    let mock_server = start_mock_server().await;
    let endpoint = format!("{}/mcp", mock_server.uri());

    // Create HTTP transport
    let mut transport = HttpTransport::new(&endpoint).await?;

    // Close the transport (no-op for HTTP, but should succeed)
    transport.close().await?;

    // Should still be "connected" since HTTP doesn't maintain connections
    assert!(transport.is_connected());

    // Should still be able to send after close
    let message = r#"{"jsonrpc":"2.0","method":"ping","id":2}"#;
    transport.send(message).await?;

    Ok(())
}

#[tokio::test]
async fn test_http_transport_invalid_endpoint() {
    // Try to connect to an invalid endpoint
    let result = HttpTransport::new("http://invalid-host-12345:9999/mcp").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_http_transport_error_response() -> Result<()> {
    let mock_server = MockServer::start().await;

    // Setup mock GET response for connectivity test
    Mock::given(method("GET"))
        .and(path("/error"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok",
            "service": "mcp"
        })))
        .mount(&mock_server)
        .await;

    // Setup mock error response for POST
    Mock::given(method("POST"))
        .and(path("/error"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let endpoint = format!("{}/error", mock_server.uri());

    // Create HTTP transport
    let mut transport = HttpTransport::new(&endpoint).await?;

    // Send should fail with 500 error
    let message = r#"{"jsonrpc":"2.0","method":"test","id":3}"#;
    let result = transport.send(message).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_http_transport_multiple_messages() -> Result<()> {
    let mock_server = start_mock_server().await;
    let endpoint = format!("{}/mcp", mock_server.uri());

    // Create HTTP transport
    let mut transport = HttpTransport::new(&endpoint).await?;

    // Send multiple messages
    for i in 1..=5 {
        let message = format!(r#"{{"jsonrpc":"2.0","method":"test","id":{i}}}"#);
        transport.send(&message).await?;
    }

    // Check statistics
    let stats = transport.stats().await;
    assert_eq!(stats.messages_sent, 5);
    assert!(stats.bytes_sent > 0);

    Ok(())
}

#[tokio::test]
async fn test_http_transport_large_message() -> Result<()> {
    let mock_server = start_mock_server().await;
    let endpoint = format!("{}/mcp", mock_server.uri());

    // Create HTTP transport
    let mut transport = HttpTransport::new(&endpoint).await?;

    // Create a large message (10KB)
    let large_content = "x".repeat(10 * 1024);
    let large_message =
        format!(r#"{{"jsonrpc":"2.0","method":"test","params":{{"data":"{large_content}"}}}}"#);

    // Send the large message
    timeout(Duration::from_secs(5), transport.send(&large_message)).await??;

    // Check stats - should be at least 10KB
    let stats = transport.stats().await;
    assert!(stats.bytes_sent >= 10 * 1024);

    Ok(())
}
