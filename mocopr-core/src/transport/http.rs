//! HTTP transport implementation for MCP.
//!
//! This module provides an HTTP-based transport layer for MCP communication.
//! Note that HTTP is inherently stateless and request-response based, which
//! doesn't perfectly align with MCP's bidirectional message flow. This
//! implementation is primarily intended for demonstration and testing purposes.
//!
//! For production MCP implementations, consider using:
//! - **Stdio transport** for process-based communication
//! - **WebSocket transport** for real-time bidirectional communication
//!
//! ## Usage
//!
//! ```rust,no_run
//! use mocopr_core::transport::http::HttpTransport;
//! use mocopr_core::transport::Transport;
//!
//! # #[tokio::main]
//! # async fn main() -> mocopr_core::Result<()> {
//! let mut transport = HttpTransport::new("http://localhost:8080/mcp").await?;
//!
//! // Send a message
//! transport.send(r#"{"jsonrpc": "2.0", "method": "ping"}"#).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Limitations
//!
//! - No bidirectional communication support
//! - Receiving messages is not implemented (would require polling or SSE)
//! - Each message requires a separate HTTP request
//! - No connection persistence or session management

use super::*;
use crate::error::TransportError;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, trace};

/// HTTP transport for MCP communication.
///
/// This transport implementation uses HTTP requests to send MCP messages.
/// It's primarily intended for demonstration and testing purposes, as HTTP's
/// request-response model doesn't naturally align with MCP's bidirectional
/// message flow.
///
/// ## Important Limitations
///
/// - **No receiving support**: The `receive()` method is not implemented since
///   HTTP is request-response based. Real implementations would need polling
///   or Server-Sent Events.
/// - **No session persistence**: Each message is a separate HTTP request.
/// - **Performance overhead**: Each message incurs HTTP request overhead.
///
/// ## Use Cases
///
/// - Testing MCP message serialization
/// - Debugging MCP protocol implementation
/// - Simple one-way communication scenarios
/// - Integration with REST-like MCP gateways
///
/// ## Examples
///
/// ```rust,no_run
/// use mocopr_core::transport::http::HttpTransport;
/// use mocopr_core::transport::Transport;
///
/// # #[tokio::main]
/// # async fn main() -> mocopr_core::Result<()> {
/// // Create HTTP transport
/// let mut transport = HttpTransport::new("http://localhost:8080/mcp").await?;
///
/// // Send a message (one-way)
/// let message = serde_json::json!({
///     "jsonrpc": "2.0",
///     "method": "tools/list",
///     "id": "test-123"
/// });
/// transport.send(&message.to_string()).await?;
///
/// // Check transport statistics
/// let stats = transport.stats().await;
/// println!("Messages sent: {}", stats.messages_sent);
/// # Ok(())
/// # }
/// ```
pub struct HttpTransport {
    client: Client,
    endpoint: String,
    stats: Arc<Mutex<TransportStats>>,
}

impl HttpTransport {
    /// Create a new HTTP transport with the specified endpoint.
    ///
    /// This method creates an HTTP client and tests connectivity to the endpoint
    /// to ensure the server is reachable. The endpoint should be a full URL
    /// where MCP messages will be posted.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - The HTTP endpoint URL (e.g., "http://localhost:8080/mcp")
    ///
    /// # Returns
    ///
    /// Returns a `Result<HttpTransport>` which is `Ok` if the endpoint is reachable.
    ///
    /// # Errors
    ///
    /// This method can fail if:
    /// - The endpoint URL is invalid
    /// - The server is not reachable
    /// - The server returns an error status code
    /// - Network connectivity issues occur
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use mocopr_core::transport::http::HttpTransport;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> mocopr_core::Result<()> {
    /// // Connect to local MCP server
    /// let transport = HttpTransport::new("http://localhost:8080/mcp").await?;
    ///
    /// // Connect to remote MCP server
    /// let transport = HttpTransport::new("https://api.example.com/mcp").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(endpoint: &str) -> Result<Self> {
        let client = Client::new();

        // Test connectivity
        let response = client.get(endpoint).send().await.map_err(|e| {
            TransportError::ConnectionFailed(format!("Failed to connect to HTTP endpoint: {e}"))
        })?;

        if !response.status().is_success() {
            return Err(TransportError::ConnectionFailed(format!(
                "HTTP endpoint returned status: {}",
                response.status()
            ))
            .into());
        }

        let stats = TransportStats {
            connection_time: Some(chrono::Utc::now()),
            ..Default::default()
        };

        Ok(Self {
            client,
            endpoint: endpoint.to_string(),
            stats: Arc::new(Mutex::new(stats)),
        })
    }

    /// Get the HTTP endpoint URL.
    ///
    /// Returns the endpoint URL that was provided when creating this transport.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use mocopr_core::transport::http::HttpTransport;
    /// # #[tokio::main]
    /// # async fn main() -> mocopr_core::Result<()> {
    /// let transport = HttpTransport::new("http://localhost:8080/mcp").await?;
    /// assert_eq!(transport.endpoint(), "http://localhost:8080/mcp");
    /// # Ok(())
    /// # }
    /// ```
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Get current transport statistics.
    ///
    /// Returns statistics about the transport usage, including message counts,
    /// byte counts, and timing information.
    ///
    /// # Returns
    ///
    /// Returns a `TransportStats` struct with current statistics.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use mocopr_core::transport::http::HttpTransport;
    /// # use mocopr_core::transport::Transport;
    /// # #[tokio::main]
    /// # async fn main() -> mocopr_core::Result<()> {
    /// let mut transport = HttpTransport::new("http://localhost:8080/mcp").await?;
    ///
    /// // Send some messages
    /// transport.send("{}").await?;
    ///
    /// // Check statistics
    /// let stats = transport.stats().await;
    /// println!("Messages sent: {}", stats.messages_sent);
    /// println!("Bytes sent: {}", stats.bytes_sent);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stats(&self) -> TransportStats {
        self.stats.lock().await.clone()
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn send(&mut self, message: &str) -> Result<()> {
        trace!("Sending message via HTTP: {}", message);

        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .body(message.to_string())
            .send()
            .await
            .map_err(|e| TransportError::SendFailed(format!("Failed to send HTTP request: {e}")))?;

        if !response.status().is_success() {
            return Err(TransportError::SendFailed(format!(
                "HTTP request failed with status: {}",
                response.status()
            ))
            .into());
        }

        let mut stats = self.stats.lock().await;
        stats.messages_sent += 1;
        stats.bytes_sent += message.len() as u64;
        stats.last_activity = Some(chrono::Utc::now());

        debug!("Message sent successfully via HTTP");
        Ok(())
    }

    async fn receive(&mut self) -> Result<Option<String>> {
        // HTTP is request-response, so receiving doesn't make sense in this context
        // This would need to be implemented with polling or SSE in a real scenario
        trace!("HTTP transport receive called - not implemented for simple HTTP");
        Ok(None)
    }

    async fn close(&mut self) -> Result<()> {
        debug!("Closing HTTP transport (no-op)");
        // HTTP doesn't maintain persistent connections in this simple implementation
        Ok(())
    }

    fn is_connected(&self) -> bool {
        // For HTTP, we assume we're always "connected" if the client exists
        true
    }

    fn transport_type(&self) -> &'static str {
        "http"
    }
}
