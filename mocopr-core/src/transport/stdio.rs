//! Standard I/O transport implementation

use super::*;
use crate::error::TransportError;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tracing::{debug, trace, warn};

/// Transport statistics
#[derive(Debug, Default)]
pub struct TransportStats {
    /// Number of messages sent
    pub messages_sent: u64,
    /// Number of messages received
    pub messages_received: u64,
    /// Number of bytes sent
    pub bytes_sent: u64,
    /// Number of bytes received
    pub bytes_received: u64,
}

/// Standard I/O transport for communicating with processes
pub struct StdioTransport {
    io: StdioIO,
    child: Option<Child>,
    stats: TransportStats,
}

/// Enum to handle different I/O types
enum StdioIO {
    /// Child process I/O
    Child {
        stdin: ChildStdin,
        stdout: BufReader<ChildStdout>,
    },
    /// Current process I/O
    Current {
        stdin: BufReader<tokio::io::Stdin>,
        stdout: tokio::io::Stdout,
    },
    /// No I/O (default state)
    None,
}

impl StdioTransport {
    /// Create a new stdio transport
    pub fn new() -> Self {
        Self {
            io: StdioIO::None,
            child: None,
            stats: TransportStats::default(),
        }
    }

    /// Create a new stdio transport from process handles
    pub fn from_process(stdin: ChildStdin, stdout: ChildStdout, child: Child) -> Self {
        Self {
            io: StdioIO::Child {
                stdin,
                stdout: BufReader::new(stdout),
            },
            child: Some(child),
            stats: TransportStats::default(),
        }
    }

    /// Create a new stdio transport by spawning a command
    pub async fn spawn<I, S>(command: &str, args: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            TransportError::ConnectionFailed(format!("Failed to spawn command '{command}': {e}"))
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            TransportError::ConnectionFailed("Failed to get stdin handle".to_string())
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            TransportError::ConnectionFailed("Failed to get stdout handle".to_string())
        })?;

        Ok(Self::from_process(stdin, stdout, child))
    }

    /// Use the current process's stdin/stdout
    pub fn current_process() -> Self {
        let stdin = BufReader::new(tokio::io::stdin());
        let stdout = tokio::io::stdout();

        Self {
            io: StdioIO::Current { stdin, stdout },
            child: None,
            stats: TransportStats::default(),
        }
    }

    /// Check if the transport is ready for communication
    pub fn is_ready(&self) -> bool {
        matches!(self.io, StdioIO::Child { .. } | StdioIO::Current { .. })
    }

    /// Get transport statistics
    pub fn stats(&self) -> &TransportStats {
        &self.stats
    }

    /// Kill the child process if it exists
    pub async fn kill(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            child.kill().await.map_err(|e| {
                Error::Transport(TransportError::ConnectionFailed(format!(
                    "Failed to kill child process: {e}"
                )))
            })?;
        }
        Ok(())
    }

    /// Wait for the child process to exit
    pub async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        if let Some(mut child) = self.child.take() {
            child.wait().await.map_err(|e| {
                Error::Transport(TransportError::ConnectionFailed(format!(
                    "Failed to wait for child: {e}"
                )))
            })
        } else {
            Err(Error::Transport(TransportError::Closed))
        }
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send(&mut self, message: &str) -> Result<()> {
        trace!("Sending message via stdio: {}", message);

        match &mut self.io {
            StdioIO::Child { stdin, .. } => {
                let line = format!("{message}\n");
                stdin.write_all(line.as_bytes()).await.map_err(|e| {
                    Error::Transport(TransportError::SendFailed(format!(
                        "Failed to write to stdin: {e}"
                    )))
                })?;

                stdin.flush().await.map_err(|e| {
                    Error::Transport(TransportError::SendFailed(format!(
                        "Failed to flush stdin: {e}"
                    )))
                })?;

                self.stats.messages_sent += 1;
                self.stats.bytes_sent += line.len() as u64;

                Ok(())
            }
            StdioIO::Current { stdout, .. } => {
                let line = format!("{message}\n");
                stdout.write_all(line.as_bytes()).await.map_err(|e| {
                    Error::Transport(TransportError::SendFailed(format!(
                        "Failed to write to stdout: {e}"
                    )))
                })?;

                stdout.flush().await.map_err(|e| {
                    Error::Transport(TransportError::SendFailed(format!(
                        "Failed to flush stdout: {e}"
                    )))
                })?;

                self.stats.messages_sent += 1;
                self.stats.bytes_sent += line.len() as u64;

                Ok(())
            }
            StdioIO::None => Err(Error::Transport(TransportError::NotReady)),
        }
    }

    async fn receive(&mut self) -> Result<Option<String>> {
        trace!("Receiving message via stdio");

        match &mut self.io {
            StdioIO::Child { stdout, .. } => {
                let mut line = String::new();
                match stdout.read_line(&mut line).await {
                    Ok(0) => {
                        // EOF - connection closed
                        Ok(None)
                    }
                    Ok(_) => {
                        // Remove trailing newline
                        if line.ends_with('\n') {
                            line.pop();
                            if line.ends_with('\r') {
                                line.pop();
                            }
                        }

                        self.stats.messages_received += 1;
                        self.stats.bytes_received += line.len() as u64;

                        trace!("Received message: {}", line);
                        Ok(Some(line))
                    }
                    Err(e) => {
                        warn!("Failed to read from stdout: {}", e);
                        Err(Error::Transport(TransportError::ReceiveFailed(format!(
                            "Failed to read from stdout: {e}"
                        ))))
                    }
                }
            }
            StdioIO::Current { stdin, .. } => {
                let mut line = String::new();
                match stdin.read_line(&mut line).await {
                    Ok(0) => {
                        // EOF - connection closed
                        Ok(None)
                    }
                    Ok(_) => {
                        // Remove trailing newline
                        if line.ends_with('\n') {
                            line.pop();
                            if line.ends_with('\r') {
                                line.pop();
                            }
                        }

                        self.stats.messages_received += 1;
                        self.stats.bytes_received += line.len() as u64;

                        trace!("Received message: {}", line);
                        Ok(Some(line))
                    }
                    Err(e) => {
                        warn!("Failed to read from stdin: {}", e);
                        Err(Error::Transport(TransportError::ReceiveFailed(format!(
                            "Failed to read from stdin: {e}"
                        ))))
                    }
                }
            }
            StdioIO::None => Err(Error::Transport(TransportError::NotReady)),
        }
    }

    async fn close(&mut self) -> Result<()> {
        debug!("Closing stdio transport");

        // Close I/O based on the type
        match std::mem::replace(&mut self.io, StdioIO::None) {
            StdioIO::Child { mut stdin, .. } => {
                let _ = stdin.shutdown().await;
            }
            StdioIO::Current { stdout, .. } => {
                // stdout will be closed when dropped
                drop(stdout);
            }
            StdioIO::None => {}
        }

        // Wait for child process to exit
        if let Some(mut child) = self.child.take() {
            tokio::spawn(async move {
                let _ = child.wait().await;
            });
        }

        Ok(())
    }

    fn transport_type(&self) -> &'static str {
        "stdio"
    }

    fn is_connected(&self) -> bool {
        self.is_ready() && self.child.is_some()
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_stdio_transport_creation() {
        let transport = StdioTransport::new();
        assert!(!transport.is_ready());
        assert_eq!(transport.transport_type(), "stdio");
    }

    #[tokio::test]
    async fn test_stdio_transport_stats() {
        let transport = StdioTransport::new();
        let stats = transport.stats();
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
    }

    #[tokio::test]
    async fn test_stdio_transport_message_serialization() {
        let _transport = StdioTransport::new();

        // Test valid JSON
        let valid_message = json!({
            "jsonrpc": "2.0",
            "method": "test",
            "id": 1
        });

        // We can't actually send/receive without a running process,
        // but we can test the structure
        assert!(valid_message.is_object());
        assert_eq!(valid_message["jsonrpc"], "2.0");
    }

    #[tokio::test]
    async fn test_stdio_transport_error_conditions() {
        let transport = StdioTransport::new();

        // Test that transport is not ready initially
        assert!(!transport.is_ready());

        // Test stats on unconnected transport
        let stats = transport.stats();
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
    }

    #[tokio::test]
    async fn test_transport_stats_fields() {
        let stats = TransportStats {
            messages_sent: 10,
            messages_received: 15,
            bytes_sent: 1024,
            bytes_received: 2048,
        };

        assert_eq!(stats.messages_sent, 10);
        assert_eq!(stats.messages_received, 15);
        assert_eq!(stats.bytes_sent, 1024);
        assert_eq!(stats.bytes_received, 2048);
    }

    #[tokio::test]
    async fn test_transport_factory() {
        // Test stdio transport creation
        let transport =
            super::super::TransportFactory::create(super::super::TransportConfig::Stdio).await;
        assert!(transport.is_ok());

        let transport = transport.unwrap();
        assert_eq!(transport.transport_type(), "stdio");
    }

    #[tokio::test]
    async fn test_transport_spawn_invalid_command() {
        // Test spawning an invalid command
        let result = StdioTransport::spawn("nonexistent_command_12345", Vec::<String>::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_transport_config_types() {
        // Test different transport config types
        let stdio_config = super::super::TransportConfig::Stdio;
        let ws_config = super::super::TransportConfig::WebSocket {
            url: "ws://localhost:8080".to_string(),
        };
        let http_config = super::super::TransportConfig::Http {
            url: "http://localhost:8080".to_string(),
        };

        // Just verify they can be created - actual connection testing would require running servers
        assert!(matches!(stdio_config, super::super::TransportConfig::Stdio));
        assert!(matches!(
            ws_config,
            super::super::TransportConfig::WebSocket { .. }
        ));
        assert!(matches!(
            http_config,
            super::super::TransportConfig::Http { .. }
        ));
    }
}
