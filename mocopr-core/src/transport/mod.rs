//! Transport abstraction for MCP communication
//!
//! This module provides transport-agnostic communication for MCP.
//! Supports stdio, HTTP, WebSocket, and other transports.

use crate::{Error, Result};
use async_trait::async_trait;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

pub mod http;
pub mod stdio;
pub mod websocket;

/// Transport abstraction for MCP communication
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send a message
    async fn send(&mut self, message: &str) -> Result<()>;

    /// Receive a message
    async fn receive(&mut self) -> Result<Option<String>>;

    /// Close the transport
    async fn close(&mut self) -> Result<()>;

    /// Check if the transport is connected
    fn is_connected(&self) -> bool;

    /// Get the transport type name
    fn transport_type(&self) -> &'static str;
}

/// Transport configuration
#[derive(Debug)]
pub enum TransportConfig {
    /// Standard I/O transport
    Stdio,
    /// WebSocket transport with URL
    WebSocket {
        /// WebSocket URL
        url: String,
    },
    /// HTTP transport with URL
    Http {
        /// HTTP URL
        url: String,
    },
    /// Custom transport configuration
    Custom(Box<dyn CustomTransportConfig>),
}

/// Trait for custom transport configurations
pub trait CustomTransportConfig: std::fmt::Debug + Send + Sync {
    /// Get the transport type name
    fn transport_type(&self) -> &'static str;
}

/// Message stream type
pub type MessageStream = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;

/// Transport factory for creating transports
pub struct TransportFactory;

impl TransportFactory {
    /// Create a transport from configuration
    pub async fn create(config: TransportConfig) -> Result<Box<dyn Transport>> {
        match config {
            TransportConfig::Stdio => Ok(Box::new(stdio::StdioTransport::new())),
            TransportConfig::WebSocket { url } => {
                Ok(Box::new(websocket::WebSocketTransport::new(&url).await?))
            }
            TransportConfig::Http { url } => Ok(Box::new(http::HttpTransport::new(&url).await?)),
            TransportConfig::Custom(_) => {
                Err(Error::internal("Custom transports not yet implemented"))
            }
        }
    }
}

/// Transport message for internal use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportMessage {
    /// Message data
    pub data: String,
    /// Message timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl TransportMessage {
    /// Creates a new transport message with current timestamp
    ///
    /// # Arguments
    /// * `data` - Message data
    pub fn new(data: String) -> Self {
        Self {
            data,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Transport statistics
#[derive(Debug, Clone, Default)]
pub struct TransportStats {
    /// Number of messages sent
    pub messages_sent: u64,
    /// Number of messages received
    pub messages_received: u64,
    /// Number of bytes sent
    pub bytes_sent: u64,
    /// Number of bytes received
    pub bytes_received: u64,
    /// Connection establishment time
    pub connection_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Last activity timestamp
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
}
