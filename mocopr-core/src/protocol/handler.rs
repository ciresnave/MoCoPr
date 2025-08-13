//! Protocol message handlers

use super::*;
use crate::{Error, Result};
use async_trait::async_trait;

/// Trait for handling MCP protocol messages
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle an initialize request
    async fn handle_initialize(&self, request: InitializeRequest) -> Result<InitializeResponse>;

    /// Handle an initialized notification
    async fn handle_initialized(&self, _notification: InitializedNotification) -> Result<()> {
        Ok(())
    }

    /// Handle a ping request
    async fn handle_ping(&self, request: PingRequest) -> Result<PingResponse> {
        Ok(PingResponse {
            message: request.message,
        })
    }

    /// Handle resources/list request
    async fn handle_resources_list(
        &self,
        _request: ResourcesListRequest,
    ) -> Result<ResourcesListResponse> {
        Err(Error::MethodNotFound("resources/list".to_string()))
    }

    /// Handle resources/read request
    async fn handle_resources_read(
        &self,
        _request: ResourcesReadRequest,
    ) -> Result<ResourcesReadResponse> {
        Err(Error::MethodNotFound("resources/read".to_string()))
    }

    /// Handle resources/subscribe request
    async fn handle_resources_subscribe(
        &self,
        _request: ResourcesSubscribeRequest,
    ) -> Result<ResourcesSubscribeResponse> {
        Err(Error::MethodNotFound("resources/subscribe".to_string()))
    }

    /// Handle resources/unsubscribe request
    async fn handle_resources_unsubscribe(
        &self,
        _request: ResourcesUnsubscribeRequest,
    ) -> Result<ResourcesUnsubscribeResponse> {
        Err(Error::MethodNotFound("resources/unsubscribe".to_string()))
    }

    /// Handle tools/list request
    async fn handle_tools_list(&self, _request: ToolsListRequest) -> Result<ToolsListResponse> {
        Err(Error::MethodNotFound("tools/list".to_string()))
    }

    /// Handle tools/call request
    async fn handle_tools_call(&self, _request: ToolsCallRequest) -> Result<ToolsCallResponse> {
        Err(Error::MethodNotFound("tools/call".to_string()))
    }

    /// Handle prompts/list request
    async fn handle_prompts_list(
        &self,
        _request: PromptsListRequest,
    ) -> Result<PromptsListResponse> {
        Err(Error::MethodNotFound("prompts/list".to_string()))
    }

    /// Handle prompts/get request
    async fn handle_prompts_get(&self, _request: PromptsGetRequest) -> Result<PromptsGetResponse> {
        Err(Error::MethodNotFound("prompts/get".to_string()))
    }

    /// Handle logging/setLevel request
    async fn handle_logging_set_level(
        &self,
        _request: LoggingSetLevelRequest,
    ) -> Result<LoggingSetLevelResponse> {
        Ok(LoggingSetLevelResponse {
            meta: ResponseMetadata { _meta: None },
        })
    }

    /// Handle sampling/createMessage request (client capability)
    async fn handle_sampling_create_message(
        &self,
        _request: CreateMessageRequest,
    ) -> Result<CreateMessageResponse> {
        Err(Error::MethodNotFound("sampling/createMessage".to_string()))
    }

    /// Handle roots/list request (client capability)
    async fn handle_roots_list(&self, _request: RootsListRequest) -> Result<RootsListResponse> {
        Err(Error::MethodNotFound("roots/list".to_string()))
    }

    /// Handle progress notification
    async fn handle_progress_notification(
        &self,
        _notification: ProgressNotification,
    ) -> Result<()> {
        Ok(())
    }

    /// Handle logging notification
    async fn handle_logging_notification(&self, _notification: LoggingNotification) -> Result<()> {
        Ok(())
    }

    /// Handle cancelled notification
    async fn handle_cancelled_notification(
        &self,
        _notification: CancelledNotification,
    ) -> Result<()> {
        Ok(())
    }

    /// Handle resources updated notification
    async fn handle_resources_updated_notification(
        &self,
        _notification: ResourcesUpdatedNotification,
    ) -> Result<()> {
        Ok(())
    }

    /// Handle tools updated notification
    async fn handle_tools_updated_notification(
        &self,
        _notification: ToolsListChangedNotification,
    ) -> Result<()> {
        Ok(())
    }

    /// Handle prompts updated notification
    async fn handle_prompts_updated_notification(
        &self,
        _notification: PromptsListChangedNotification,
    ) -> Result<()> {
        Ok(())
    }

    /// Handle roots updated notification
    async fn handle_roots_updated_notification(
        &self,
        _notification: RootsListChangedNotification,
    ) -> Result<()> {
        Ok(())
    }

    /// Handle custom/unknown requests
    async fn handle_custom_request(
        &self,
        method: &str,
        _params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        Err(Error::MethodNotFound(method.to_string()))
    }

    /// Handle custom/unknown notifications
    async fn handle_custom_notification(
        &self,
        method: &str,
        _params: Option<serde_json::Value>,
    ) -> Result<()> {
        // By default, ignore unknown notifications
        tracing::debug!("Received unknown notification: {}", method);
        Ok(())
    }
}

/// Default implementation of MessageHandler
pub struct DefaultMessageHandler {
    /// Information about the server implementation
    pub server_info: Implementation,
    /// Capabilities supported by the server
    pub capabilities: ServerCapabilities,
}

impl DefaultMessageHandler {
    /// Creates a new default message handler with the specified server info and capabilities
    pub fn new(server_info: Implementation, capabilities: ServerCapabilities) -> Self {
        Self {
            server_info,
            capabilities,
        }
    }
}

#[async_trait]
impl MessageHandler for DefaultMessageHandler {
    async fn handle_initialize(&self, request: InitializeRequest) -> Result<InitializeResponse> {
        // Validate protocol version
        if !Protocol::is_version_supported(&request.protocol_version) {
            return Err(Error::InvalidRequest(format!(
                "Unsupported protocol version: {}",
                request.protocol_version
            )));
        }

        Ok(InitializeResponse {
            protocol_version: Protocol::latest_version().to_string(),
            capabilities: self.capabilities.clone(),
            server_info: self.server_info.clone(),
            instructions: None,
        })
    }
}

/// Builder for creating message handlers with chained configuration
pub struct MessageHandlerBuilder {
    server_info: Option<Implementation>,
    capabilities: ServerCapabilities,
}

impl MessageHandlerBuilder {
    /// Creates a new message handler builder with default settings
    pub fn new() -> Self {
        Self {
            server_info: None,
            capabilities: ServerCapabilities::default(),
        }
    }

    /// Sets the server information for the message handler
    pub fn with_server_info(mut self, name: String, version: String) -> Self {
        self.server_info = Some(Implementation { name, version });
        self
    }

    /// Enables logging capabilities in the server
    pub fn with_logging(mut self) -> Self {
        self.capabilities = self.capabilities.with_logging();
        self
    }

    /// Configures resource handling capabilities
    ///
    /// * `list_changed` - Whether the server supports resource list change notifications
    /// * `subscribe` - Whether the server supports resource subscription
    pub fn with_resources(mut self, list_changed: bool, subscribe: bool) -> Self {
        self.capabilities = self.capabilities.with_resources(list_changed, subscribe);
        self
    }

    /// Configures tool capabilities
    ///
    /// * `list_changed` - Whether the server supports tool list change notifications
    pub fn with_tools(mut self, list_changed: bool) -> Self {
        self.capabilities = self.capabilities.with_tools(list_changed);
        self
    }

    /// Configures prompt capabilities
    ///
    /// * `list_changed` - Whether the server supports prompt list change notifications
    pub fn with_prompts(mut self, list_changed: bool) -> Self {
        self.capabilities = self.capabilities.with_prompts(list_changed);
        self
    }

    /// Builds the message handler with the configured settings
    pub fn build(self) -> Result<DefaultMessageHandler> {
        let server_info = self
            .server_info
            .ok_or_else(|| Error::InvalidRequest("Server info is required".to_string()))?;

        Ok(DefaultMessageHandler::new(server_info, self.capabilities))
    }
}

impl Default for MessageHandlerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
