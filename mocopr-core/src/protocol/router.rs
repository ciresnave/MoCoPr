//! Message routing for MCP protocol

use super::*;
use crate::Result;
use std::sync::Arc;

/// Message router for dispatching MCP messages to handlers
pub struct MessageRouter {
    handler: Arc<dyn MessageHandler>,
}

impl MessageRouter {
    /// Create a new message router with the given handler
    pub fn new(handler: Arc<dyn MessageHandler>) -> Self {
        Self { handler }
    }

    /// Route a JSON-RPC message to the appropriate handler
    pub async fn route_message(&self, message: JsonRpcMessage) -> Result<Option<JsonRpcMessage>> {
        match message {
            JsonRpcMessage::Request(request) => {
                let response = self.route_request(request).await?;
                Ok(Some(JsonRpcMessage::Response(response)))
            }
            JsonRpcMessage::Notification(notification) => {
                self.route_notification(notification).await?;
                Ok(None)
            }
            JsonRpcMessage::Response(_) => {
                // Responses are handled by the caller, not routed
                Ok(None)
            }
        }
    }

    /// Route a request message
    async fn route_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let result = self.dispatch_request(&request).await;

        match result {
            Ok(response_data) => Ok(Protocol::create_response(
                request.id,
                Some(response_data),
                None,
            )),
            Err(error) => {
                let jsonrpc_error = Protocol::error_to_jsonrpc(&error);
                Ok(Protocol::create_response(
                    request.id,
                    None,
                    Some(jsonrpc_error),
                ))
            }
        }
    }

    /// Route a notification message
    async fn route_notification(&self, notification: JsonRpcNotification) -> Result<()> {
        self.dispatch_notification(&notification).await
    }

    /// Dispatch a request to the appropriate handler method
    async fn dispatch_request(&self, request: &JsonRpcRequest) -> Result<serde_json::Value> {
        match request.method.as_str() {
            "initialize" => {
                let req: InitializeRequest = self.deserialize_params(request.params.as_ref())?;
                let response = self.handler.handle_initialize(req).await?;
                Ok(serde_json::to_value(&response)?)
            }
            "ping" => {
                let req: PingRequest = self.deserialize_params(request.params.as_ref())?;
                let response = self.handler.handle_ping(req).await?;
                Ok(serde_json::to_value(&response)?)
            }
            "resources/list" => {
                let req: ResourcesListRequest = self.deserialize_params(request.params.as_ref())?;
                let response = self.handler.handle_resources_list(req).await?;
                Ok(serde_json::to_value(&response)?)
            }
            "resources/read" => {
                let req: ResourcesReadRequest = self.deserialize_params(request.params.as_ref())?;
                let response = self.handler.handle_resources_read(req).await?;
                Ok(serde_json::to_value(&response)?)
            }
            "resources/subscribe" => {
                let req: ResourcesSubscribeRequest =
                    self.deserialize_params(request.params.as_ref())?;
                let response = self.handler.handle_resources_subscribe(req).await?;
                Ok(serde_json::to_value(&response)?)
            }
            "resources/unsubscribe" => {
                let req: ResourcesUnsubscribeRequest =
                    self.deserialize_params(request.params.as_ref())?;
                let response = self.handler.handle_resources_unsubscribe(req).await?;
                Ok(serde_json::to_value(&response)?)
            }
            "tools/list" => {
                let req: ToolsListRequest = self.deserialize_params(request.params.as_ref())?;
                let response = self.handler.handle_tools_list(req).await?;
                Ok(serde_json::to_value(&response)?)
            }
            "tools/call" => {
                let req: ToolsCallRequest = self.deserialize_params(request.params.as_ref())?;
                let response = self.handler.handle_tools_call(req).await?;
                Ok(serde_json::to_value(&response)?)
            }
            "prompts/list" => {
                let req: PromptsListRequest = self.deserialize_params(request.params.as_ref())?;
                let response = self.handler.handle_prompts_list(req).await?;
                Ok(serde_json::to_value(&response)?)
            }
            "prompts/get" => {
                let req: PromptsGetRequest = self.deserialize_params(request.params.as_ref())?;
                let response = self.handler.handle_prompts_get(req).await?;
                Ok(serde_json::to_value(&response)?)
            }
            "logging/setLevel" => {
                let req: LoggingSetLevelRequest =
                    self.deserialize_params(request.params.as_ref())?;
                let response = self.handler.handle_logging_set_level(req).await?;
                Ok(serde_json::to_value(&response)?)
            }
            "sampling/createMessage" => {
                let req: CreateMessageRequest = self.deserialize_params(request.params.as_ref())?;
                let response = self.handler.handle_sampling_create_message(req).await?;
                Ok(serde_json::to_value(&response)?)
            }
            "roots/list" => {
                let req: RootsListRequest = self.deserialize_params(request.params.as_ref())?;
                let response = self.handler.handle_roots_list(req).await?;
                Ok(serde_json::to_value(&response)?)
            }
            method => {
                // Handle custom methods
                let response = self
                    .handler
                    .handle_custom_request(method, request.params.clone())
                    .await?;
                Ok(response)
            }
        }
    }

    /// Dispatch a notification to the appropriate handler method
    async fn dispatch_notification(&self, notification: &JsonRpcNotification) -> Result<()> {
        match notification.method.as_str() {
            "initialized" => {
                let notif: InitializedNotification =
                    self.deserialize_params(notification.params.as_ref())?;
                self.handler.handle_initialized(notif).await
            }
            "notifications/progress" => {
                let notif: ProgressNotification =
                    self.deserialize_params(notification.params.as_ref())?;
                self.handler.handle_progress_notification(notif).await
            }
            "notifications/message" => {
                let notif: LoggingNotification =
                    self.deserialize_params(notification.params.as_ref())?;
                self.handler.handle_logging_notification(notif).await
            }
            "notifications/cancelled" => {
                let notif: CancelledNotification =
                    self.deserialize_params(notification.params.as_ref())?;
                self.handler.handle_cancelled_notification(notif).await
            }
            "notifications/resources/updated" => {
                let notif: ResourcesUpdatedNotification =
                    self.deserialize_params(notification.params.as_ref())?;
                self.handler
                    .handle_resources_updated_notification(notif)
                    .await
            }
            "notifications/tools/updated" => {
                let notif: ToolsListChangedNotification =
                    self.deserialize_params(notification.params.as_ref())?;
                self.handler.handle_tools_updated_notification(notif).await
            }
            "notifications/prompts/updated" => {
                let notif: PromptsListChangedNotification =
                    self.deserialize_params(notification.params.as_ref())?;
                self.handler
                    .handle_prompts_updated_notification(notif)
                    .await
            }
            "notifications/roots/updated" => {
                let notif: RootsListChangedNotification =
                    self.deserialize_params(notification.params.as_ref())?;
                self.handler.handle_roots_updated_notification(notif).await
            }
            method => {
                // Handle custom notifications
                self.handler
                    .handle_custom_notification(method, notification.params.clone())
                    .await
            }
        }
    }

    /// Deserialize request/notification parameters
    fn deserialize_params<T: serde::de::DeserializeOwned>(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<T> {
        match params {
            Some(value) => Ok(serde_json::from_value(value.clone())?),
            None => Ok(serde_json::from_value(serde_json::Value::Object(
                serde_json::Map::new(),
            ))?),
        }
    }
}
