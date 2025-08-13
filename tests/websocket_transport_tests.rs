//! Integration tests for WebSocket transport
//!
//! These tests verify that the WebSocket transport layer works correctly
//! with real WebSocket servers and proper MCP message handling.

use anyhow::Result;
use futures::StreamExt;
use mocopr_core::transport::{Transport, websocket::WebSocketTransport};
use tokio::net::TcpListener;
use tokio::time::{Duration, timeout};

// Helper function to start a test WebSocket server
async fn start_test_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                if let Ok(ws_stream) = tokio_tungstenite::accept_async(stream).await {
                    let (mut write, mut read) = ws_stream.split();

                    // Echo server implementation
                    while let Some(msg) = futures::StreamExt::next(&mut read).await {
                        if let Ok(msg) = msg {
                            match msg {
                                tokio_tungstenite::tungstenite::Message::Text(text) => {
                                    // Echo back with "Echo: " prefix
                                    let response = format!("Echo: {text}");
                                    let _ = futures::SinkExt::send(
                                        &mut write,
                                        tokio_tungstenite::tungstenite::Message::Text(response),
                                    )
                                    .await;
                                }
                                tokio_tungstenite::tungstenite::Message::Close(_) => break,
                                _ => {}
                            }
                        } else {
                            break;
                        }
                    }
                }
            });
        }
    });

    // Give the server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    port
}

#[tokio::test]
async fn test_websocket_connection() -> Result<()> {
    // Start test server
    let port = start_test_server().await;
    let url = format!("ws://127.0.0.1:{port}");

    // Create WebSocket transport
    let transport = WebSocketTransport::new(&url).await?;

    // Verify connection properties
    assert!(transport.is_connected());
    assert_eq!(transport.url(), &url);
    assert_eq!(transport.transport_type(), "websocket");

    Ok(())
}

#[tokio::test]
async fn test_websocket_send_receive() -> Result<()> {
    // Start test server
    let port = start_test_server().await;
    let url = format!("ws://127.0.0.1:{port}");

    // Create WebSocket transport
    let mut transport = WebSocketTransport::new(&url).await?;

    // Send a message
    let test_message = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
    transport.send(test_message).await?;

    // Receive echo response
    if let Some(received) = timeout(Duration::from_secs(2), transport.receive()).await?? {
        assert_eq!(received, format!("Echo: {test_message}"));
    } else {
        panic!("Expected echo response but got None");
    }

    // Check statistics
    let stats = transport.stats();
    assert_eq!(stats.messages_sent, 1);
    assert_eq!(stats.messages_received, 1);
    assert!(stats.bytes_sent > 0);
    assert!(stats.bytes_received > 0);

    Ok(())
}

#[tokio::test]
async fn test_websocket_close() -> Result<()> {
    // Start test server
    let port = start_test_server().await;
    let url = format!("ws://127.0.0.1:{port}");

    // Create and verify WebSocket transport
    let mut transport = WebSocketTransport::new(&url).await?;
    assert!(transport.is_connected());

    // Close the connection
    transport.close().await?;
    assert!(!transport.is_connected());

    // Sending after close should fail
    let result = transport.send("test message").await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_websocket_connection_refused() {
    // Try to connect to a port that's not listening
    let result = WebSocketTransport::new("ws://127.0.0.1:59999").await;
    assert!(result.is_err());

    // Verify it's a connection error
    if let Err(err) = result {
        let err_str = format!("{err:?}");
        assert!(err_str.contains("Failed to connect") || err_str.contains("ConnectionFailed"));
    }
}

#[tokio::test]
async fn test_websocket_reconnect() -> Result<()> {
    // Start test server
    let port = start_test_server().await;
    let url = format!("ws://127.0.0.1:{port}");

    // Create WebSocket transport
    let mut transport = WebSocketTransport::new(&url).await?;
    assert!(transport.is_connected());

    // Close the connection
    transport.close().await?;
    assert!(!transport.is_connected());

    // Reconnect
    transport.reconnect().await?;
    assert!(transport.is_connected());

    // Verify communication works after reconnect
    let test_message = r#"{"jsonrpc":"2.0","method":"ping","id":2}"#;
    transport.send(test_message).await?;

    // Receive echo response
    if let Some(received) = timeout(Duration::from_secs(2), transport.receive()).await?? {
        assert_eq!(received, format!("Echo: {test_message}"));
    } else {
        panic!("Expected echo response after reconnect but got None");
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_large_message() -> Result<()> {
    // Start test server
    let port = start_test_server().await;
    let url = format!("ws://127.0.0.1:{port}");

    // Create WebSocket transport
    let mut transport = WebSocketTransport::new(&url).await?;

    // Create a large message (10KB)
    let large_content = "x".repeat(10 * 1024);
    let large_message =
        format!(r#"{{"jsonrpc":"2.0","method":"test","params":{{"data":"{large_content}"}}}}"#);

    // Send the large message
    transport.send(&large_message).await?;

    // Receive echo response (with longer timeout)
    if let Some(received) = timeout(Duration::from_secs(5), transport.receive()).await?? {
        assert_eq!(received, format!("Echo: {large_message}"));
    } else {
        panic!("Expected echo response for large message but got None");
    }

    // Check stats - should be at least 10KB
    let stats = transport.stats();
    assert!(stats.bytes_sent >= 10 * 1024);
    assert!(stats.bytes_received >= 10 * 1024);

    Ok(())
}
