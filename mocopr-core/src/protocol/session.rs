//! Session management for MCP connections

use super::*;
use crate::{Error, Result, transport::Transport, utils::Utils};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, mpsc};
use uuid::Uuid;

/// Represents an active MCP session
pub struct Session {
    id: String,
    state: Arc<RwLock<SessionState>>,
    transport: Arc<Mutex<Box<dyn Transport>>>,
    router: MessageRouter,
    pending_requests: Arc<Mutex<HashMap<RequestId, PendingRequest>>>,
    event_sender: mpsc::UnboundedSender<SessionEvent>,
}

/// Session state information
#[derive(Debug, Clone)]
pub struct SessionState {
    /// Whether the session has been initialized
    pub initialized: bool,
    /// Information about the connected client
    pub client_info: Option<Implementation>,
    /// Information about the connected server
    pub server_info: Option<Implementation>,
    /// Capabilities supported by the client
    pub client_capabilities: Option<ClientCapabilities>,
    /// Capabilities supported by the server
    pub server_capabilities: Option<ServerCapabilities>,
    /// The MCP protocol version in use for this session
    pub protocol_version: Option<String>,
    /// Timestamp when the connection was established
    pub connected_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp of the last activity on this session
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

/// Pending request tracking
#[derive(Debug)]
struct PendingRequest {
    /// Channel sender for the pending response
    sender: tokio::sync::oneshot::Sender<Result<JsonRpcResponse>>,
    /// When the request was created
    created_at: std::time::Instant,
    /// Optional timeout duration for the request
    timeout: Option<std::time::Duration>,
}

/// Session events
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// Event triggered when a connection is established
    Connected,
    /// Event triggered when a connection is closed
    Disconnected,
    /// Event triggered when a session is successfully initialized
    Initialized {
        /// Information about the connected client
        client_info: Implementation,
    },
    /// Event triggered when a message is received
    MessageReceived {
        /// The content of the received message
        message: String,
    },
    /// Event triggered when a message is sent
    MessageSent {
        /// The content of the sent message
        message: String,
    },
    /// Event triggered when an error occurs
    Error {
        /// The error message
        error: String,
    },
}

impl Session {
    /// Create a new session
    pub fn new(
        transport: Box<dyn Transport>,
        handler: Arc<dyn MessageHandler>,
    ) -> (Self, mpsc::UnboundedReceiver<SessionEvent>) {
        let id = Uuid::new_v4().to_string();
        let router = MessageRouter::new(handler);
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let session = Self {
            id,
            state: Arc::new(RwLock::new(SessionState {
                initialized: false,
                client_info: None,
                server_info: None,
                client_capabilities: None,
                server_capabilities: None,
                protocol_version: None,
                connected_at: chrono::Utc::now(),
                last_activity: chrono::Utc::now(),
            })),
            transport: Arc::new(Mutex::new(transport)),
            router,
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
        };

        (session, event_receiver)
    }

    /// Get session ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get session state
    pub async fn state(&self) -> SessionState {
        self.state.read().await.clone()
    }

    /// Check if session is initialized
    pub async fn is_initialized(&self) -> bool {
        self.state.read().await.initialized
    }

    /// Send a request and wait for response
    pub async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let request_id = request
            .id
            .clone()
            .ok_or_else(|| Error::InvalidRequest("Request must have an ID".to_string()))?;

        let (tx, rx) = tokio::sync::oneshot::channel();

        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(
                request_id.clone(),
                PendingRequest {
                    sender: tx,
                    created_at: std::time::Instant::now(),
                    timeout: Some(std::time::Duration::from_secs(30)),
                },
            );
        }

        // Send the request
        let message = Protocol::serialize_message(&JsonRpcMessage::Request(request))?;
        self.send_message(&message).await?;

        // Wait for response
        match rx.await {
            Ok(result) => result,
            Err(_) => {
                // Remove from pending requests
                self.pending_requests.lock().await.remove(&request_id);
                Err(Error::Timeout)
            }
        }
    }

    /// Send a notification
    pub async fn send_notification(&self, notification: JsonRpcNotification) -> Result<()> {
        let message = Protocol::serialize_message(&JsonRpcMessage::Notification(notification))?;
        self.send_message(&message).await
    }

    /// Send a raw message
    async fn send_message(&self, message: &str) -> Result<()> {
        {
            let mut transport = self.transport.lock().await;
            transport.send(message).await?;
        }

        // Update last activity
        {
            let mut state = self.state.write().await;
            state.last_activity = chrono::Utc::now();
        }

        // Send event
        let _ = self.event_sender.send(SessionEvent::MessageSent {
            message: message.to_string(),
        });

        Ok(())
    }

    /// Start the session message loop
    pub async fn run(&self) -> Result<()> {
        let _ = self.event_sender.send(SessionEvent::Connected);

        loop {
            // Receive message from transport
            let message = {
                let mut transport = self.transport.lock().await;
                transport.receive().await?
            };

            let message = match message {
                Some(msg) => msg,
                None => {
                    // Connection closed
                    let _ = self.event_sender.send(SessionEvent::Disconnected);
                    break;
                }
            };

            // Update last activity
            {
                let mut state = self.state.write().await;
                state.last_activity = chrono::Utc::now();
            }

            // Send event
            let _ = self.event_sender.send(SessionEvent::MessageReceived {
                message: message.clone(),
            });

            // Process message
            if let Err(e) = self.process_message(&message).await {
                let _ = self.event_sender.send(SessionEvent::Error {
                    error: e.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Process an incoming message
    async fn process_message(&self, message: &str) -> Result<()> {
        let jsonrpc_message = Protocol::parse_message(message)?;

        match &jsonrpc_message {
            JsonRpcMessage::Response(response) => {
                self.handle_response(response).await?;
            }
            JsonRpcMessage::Request(_) | JsonRpcMessage::Notification(_) => {
                // Route to handler
                if let Some(response_message) = self.router.route_message(jsonrpc_message).await? {
                    let response_str = Protocol::serialize_message(&response_message)?;
                    self.send_message(&response_str).await?;
                }
            }
        }

        Ok(())
    }

    /// Handle incoming response
    async fn handle_response(&self, response: &JsonRpcResponse) -> Result<()> {
        if let Some(ref response_id) = response.id {
            let mut pending = self.pending_requests.lock().await;
            if let Some(pending_request) = pending.remove(response_id) {
                let _ = pending_request.sender.send(Ok(response.clone()));
            }
        }
        Ok(())
    }

    /// Initialize the session
    pub async fn initialize(
        &self,
        client_info: Implementation,
        client_capabilities: ClientCapabilities,
    ) -> Result<InitializeResponse> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Protocol::generate_request_id()),
            method: "initialize".to_string(),
            params: Some(Utils::to_json_value(&InitializeRequest {
                protocol_version: Protocol::latest_version().to_string(),
                capabilities: client_capabilities.clone(),
                client_info: client_info.clone(),
            })?),
        };

        let response = self.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(Error::Server(format!(
                "Initialize failed: {}",
                error.message
            )));
        }

        let result = response
            .result
            .ok_or_else(|| Error::Server("Initialize response missing result".to_string()))?;

        let init_response: InitializeResponse = Utils::from_json_value(result)?;

        // Update session state
        {
            let mut state = self.state.write().await;
            state.client_info = Some(client_info);
            state.client_capabilities = Some(client_capabilities);
            state.server_info = Some(init_response.server_info.clone());
            state.server_capabilities = Some(init_response.capabilities.clone());
            state.protocol_version = Some(init_response.protocol_version.clone());
            state.initialized = true;
        }

        // Send initialized notification
        let initialized_notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "initialized".to_string(),
            params: Some(Utils::to_json_value(&InitializedNotification {})?),
        };

        self.send_notification(initialized_notification).await?;

        // Send event
        let _ = self.event_sender.send(SessionEvent::Initialized {
            client_info: init_response.server_info.clone(),
        });

        Ok(init_response)
    }

    /// Close the session
    pub async fn close(&self) -> Result<()> {
        let mut transport = self.transport.lock().await;
        transport.close().await?;
        let _ = self.event_sender.send(SessionEvent::Disconnected);
        Ok(())
    }

    /// Check if transport is connected
    pub async fn is_connected(&self) -> bool {
        let transport = self.transport.lock().await;
        transport.is_connected()
    }

    /// Clean up expired pending requests
    pub async fn cleanup_expired_requests(&self) {
        let mut pending = self.pending_requests.lock().await;
        let now = std::time::Instant::now();

        let mut timed_out_requests = Vec::new();

        pending.retain(|id, request| {
            if let Some(timeout) = request.timeout {
                if now.duration_since(request.created_at) > timeout {
                    timed_out_requests.push(id.clone());
                    false
                } else {
                    true
                }
            } else {
                true
            }
        });

        // Send timeout errors for timed out requests
        for id in timed_out_requests {
            if let Some(request) = pending.remove(&id) {
                let _ = request.sender.send(Err(Error::Timeout));
            }
        }
    }
}

impl Default for SessionState {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            initialized: false,
            client_info: None,
            server_info: None,
            client_capabilities: None,
            server_capabilities: None,
            protocol_version: None,
            connected_at: now,
            last_activity: now,
        }
    }
}
