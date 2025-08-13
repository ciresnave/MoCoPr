//! WebSocket transport implementation

use super::*;
use crate::error::TransportError;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use tracing::{debug, error, trace};

/// WebSocket transport for MCP communication
pub struct WebSocketTransport {
    sink: Option<SplitSink<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, Message>>,
    stream: Option<SplitStream<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>>,
    url: String,
    stats: TransportStats,
}

impl WebSocketTransport {
    /// Create a new WebSocket transport
    pub async fn new(url: &str) -> Result<Self> {
        let (ws_stream, _) = connect_async(url).await.map_err(|e| {
            TransportError::ConnectionFailed(format!("Failed to connect to WebSocket: {e}"))
        })?;

        let (sink, stream) = ws_stream.split();

        let stats = TransportStats {
            connection_time: Some(chrono::Utc::now()),
            ..Default::default()
        };

        Ok(Self {
            sink: Some(sink),
            stream: Some(stream),
            url: url.to_string(),
            stats,
        })
    }

    /// Get the WebSocket URL
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get transport statistics
    pub fn stats(&self) -> &TransportStats {
        &self.stats
    }

    /// Reconnect to the WebSocket
    pub async fn reconnect(&mut self) -> Result<()> {
        debug!("Reconnecting to WebSocket: {}", self.url);

        self.close().await?;

        let (ws_stream, _) = connect_async(&self.url).await.map_err(|e| {
            TransportError::ConnectionFailed(format!("Failed to reconnect to WebSocket: {e}"))
        })?;

        let (sink, stream) = ws_stream.split();

        self.sink = Some(sink);
        self.stream = Some(stream);
        self.stats.connection_time = Some(chrono::Utc::now());

        Ok(())
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    async fn send(&mut self, message: &str) -> Result<()> {
        trace!("Sending message via WebSocket: {}", message);

        if let Some(sink) = &mut self.sink {
            sink.send(Message::Text(message.to_string()))
                .await
                .map_err(|e| {
                    TransportError::SendFailed(format!("Failed to send WebSocket message: {e}"))
                })?;

            self.stats.messages_sent += 1;
            self.stats.bytes_sent += message.len() as u64;
            self.stats.last_activity = Some(chrono::Utc::now());

            debug!("Message sent successfully via WebSocket");
            Ok(())
        } else {
            Err(TransportError::NotReady.into())
        }
    }

    async fn receive(&mut self) -> Result<Option<String>> {
        trace!("Receiving message via WebSocket");

        if let Some(stream) = &mut self.stream {
            match stream.next().await {
                Some(Ok(Message::Text(text))) => {
                    self.stats.messages_received += 1;
                    self.stats.bytes_received += text.len() as u64;
                    self.stats.last_activity = Some(chrono::Utc::now());

                    debug!("Received message via WebSocket: {}", text);
                    Ok(Some(text))
                }
                Some(Ok(Message::Binary(data))) => {
                    // Convert binary to string (UTF-8)
                    match String::from_utf8(data) {
                        Ok(text) => {
                            self.stats.messages_received += 1;
                            self.stats.bytes_received += text.len() as u64;
                            self.stats.last_activity = Some(chrono::Utc::now());

                            debug!("Received binary message via WebSocket: {}", text);
                            Ok(Some(text))
                        }
                        Err(e) => {
                            error!("Failed to decode binary WebSocket message: {}", e);
                            Err(TransportError::ReceiveFailed(format!(
                                "Failed to decode binary message: {e}"
                            ))
                            .into())
                        }
                    }
                }
                Some(Ok(Message::Close(_))) => {
                    debug!("WebSocket connection closed by peer");
                    Ok(None)
                }
                Some(Ok(Message::Ping(data))) => {
                    // Send pong response
                    if let Some(sink) = &mut self.sink {
                        let _ = sink.send(Message::Pong(data)).await;
                    }
                    // Continue receiving
                    self.receive().await
                }
                Some(Ok(Message::Pong(_))) => {
                    // Ignore pong messages
                    self.receive().await
                }
                Some(Ok(Message::Frame(_))) => {
                    // Ignore raw frames (should not occur in normal usage)
                    self.receive().await
                }
                Some(Err(e)) => {
                    error!("WebSocket error: {}", e);
                    Err(TransportError::ReceiveFailed(format!("WebSocket error: {e}")).into())
                }
                None => {
                    debug!("WebSocket stream ended");
                    Ok(None)
                }
            }
        } else {
            Err(TransportError::NotReady.into())
        }
    }

    async fn close(&mut self) -> Result<()> {
        debug!("Closing WebSocket transport");

        if let Some(mut sink) = self.sink.take() {
            let _ = sink.send(Message::Close(None)).await;
            let _ = sink.close().await;
        }

        self.stream = None;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.sink.is_some() && self.stream.is_some()
    }

    fn transport_type(&self) -> &'static str {
        "websocket"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{SinkExt, StreamExt};
    use std::time::Duration;
    use tokio::net::{TcpListener, TcpStream};
    use tokio_tungstenite::{accept_async, tungstenite::Message};

    /// Helper function to start a test WebSocket server
    async fn start_test_server() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(handle_connection(stream));
            }
        });

        port
    }

    async fn handle_connection(stream: TcpStream) {
        let ws_stream = accept_async(stream)
            .await
            .expect("Failed to accept WebSocket");
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Echo the message back
                    let response = format!("Echo: {}", text);
                    if ws_sender.send(Message::Text(response)).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Close(_)) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
    }

    #[tokio::test]
    async fn test_websocket_connection() {
        let port = start_test_server().await;
        let url = format!("ws://127.0.0.1:{}", port);

        // Give the server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Add timeout to prevent test from hanging
        let transport = tokio::time::timeout(Duration::from_secs(5), WebSocketTransport::new(&url))
            .await
            .expect("Test timed out")
            .expect("Failed to create WebSocket transport");

        assert!(transport.is_connected());
        assert_eq!(transport.url(), &url);
        assert_eq!(transport.transport_type(), "websocket");
    }

    #[tokio::test]
    async fn test_websocket_send_receive() {
        let port = start_test_server().await;
        let url = format!("ws://127.0.0.1:{}", port);

        // Give the server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Add timeout for connection
        let mut transport =
            tokio::time::timeout(Duration::from_secs(5), WebSocketTransport::new(&url))
                .await
                .expect("Connection timed out")
                .expect("Failed to create WebSocket transport");

        // Send a message with timeout
        let test_message = "Hello, WebSocket!";
        tokio::time::timeout(Duration::from_secs(5), transport.send(test_message))
            .await
            .expect("Send timed out")
            .expect("Failed to send message");

        // Receive the echo with timeout
        let received = tokio::time::timeout(Duration::from_secs(5), transport.receive())
            .await
            .expect("Receive timed out")
            .expect("Failed to receive message");

        assert!(received.is_some());
        let received_text = received.unwrap();
        assert_eq!(received_text, format!("Echo: {}", test_message));

        // Check stats
        let stats = transport.stats();
        assert_eq!(stats.messages_sent, 1);
        assert_eq!(stats.messages_received, 1);
        assert!(stats.bytes_sent > 0);
        assert!(stats.bytes_received > 0);
    }

    #[tokio::test]
    async fn test_websocket_close() {
        let port = start_test_server().await;
        let url = format!("ws://127.0.0.1:{}", port);

        // Give the server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Add timeout for connection
        let mut transport =
            tokio::time::timeout(Duration::from_secs(5), WebSocketTransport::new(&url))
                .await
                .expect("Connection timed out")
                .expect("Failed to create WebSocket transport");
        assert!(transport.is_connected());

        // Add timeout for close
        tokio::time::timeout(Duration::from_secs(5), transport.close())
            .await
            .expect("Close timed out")
            .expect("Failed to close transport");
        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn test_websocket_invalid_url() {
        // Add timeout to prevent hanging on connection attempt
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            WebSocketTransport::new("ws://invalid-host:99999"),
        )
        .await
        .expect("Test timed out");

        assert!(result.is_err());

        if let Err(Error::Transport(TransportError::ConnectionFailed(msg))) = result {
            assert!(msg.contains("Failed to connect to WebSocket"));
        } else {
            panic!("Expected ConnectionFailed error");
        }
    }

    #[tokio::test]
    async fn test_websocket_stats_tracking() {
        let port = start_test_server().await;
        let url = format!("ws://127.0.0.1:{}", port);

        // Give the server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Add timeout for connection
        let mut transport =
            tokio::time::timeout(Duration::from_secs(5), WebSocketTransport::new(&url))
                .await
                .expect("Connection timed out")
                .expect("Failed to create WebSocket transport");

        // Initial stats
        let stats = transport.stats();
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
        assert!(stats.connection_time.is_some());

        // Send multiple messages with timeouts
        for i in 0..3 {
            let message = format!("Message {}", i);
            tokio::time::timeout(Duration::from_secs(5), transport.send(&message))
                .await
                .expect("Send timed out")
                .expect("Failed to send message");

            tokio::time::timeout(Duration::from_secs(5), transport.receive())
                .await
                .expect("Receive timed out")
                .expect("Failed to receive message"); // Receive echo
        }

        // Check final stats
        let stats = transport.stats();
        assert_eq!(stats.messages_sent, 3);
        assert_eq!(stats.messages_received, 3);
        assert!(stats.bytes_sent > 0);
        assert!(stats.bytes_received > 0);
        assert!(stats.last_activity.is_some());
    }

    #[tokio::test]
    async fn test_websocket_large_message() {
        let port = start_test_server().await;
        let url = format!("ws://127.0.0.1:{}", port);

        // Give the server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Add timeout for connection
        let mut transport =
            tokio::time::timeout(Duration::from_secs(5), WebSocketTransport::new(&url))
                .await
                .expect("Connection timed out")
                .expect("Failed to create WebSocket transport");

        // Create a large message (10KB)
        let large_message = "x".repeat(10 * 1024);

        // Add timeout for send
        tokio::time::timeout(
            Duration::from_secs(10), // Longer timeout for large message
            transport.send(&large_message),
        )
        .await
        .expect("Send timed out")
        .expect("Failed to send message");

        // Add timeout for receive
        let received = tokio::time::timeout(
            Duration::from_secs(10), // Longer timeout for large message
            transport.receive(),
        )
        .await
        .expect("Receive timed out")
        .expect("Failed to receive message")
        .unwrap();

        assert_eq!(received, format!("Echo: {}", large_message));

        let stats = transport.stats();
        assert!(stats.bytes_sent >= 10 * 1024);
    }
}
