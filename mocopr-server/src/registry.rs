//! Registry for managing server capabilities

use crate::handlers::*;
use mocopr_core::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry for resource handlers
#[derive(Clone)]
pub struct ResourceRegistry {
    handlers: Arc<RwLock<HashMap<String, Box<dyn ResourceHandler>>>>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a resource handler
    pub fn register(&mut self, handler: Box<dyn ResourceHandler>) {
        let uri = futures::executor::block_on(async { handler.resource().await.uri.to_string() });

        futures::executor::block_on(async {
            self.handlers.write().await.insert(uri, handler);
        });
    }

    /// List all resources
    pub async fn list_resources(
        &self,
        request: ResourcesListRequest,
    ) -> Result<ResourcesListResponse> {
        let handlers = self.handlers.read().await;
        let mut resources = Vec::new();

        for handler in handlers.values() {
            resources.push(handler.resource().await);
        }

        // Apply pagination if cursor is provided
        let start_index = if let Some(cursor) = &request.pagination.cursor {
            cursor.parse::<usize>().unwrap_or(0)
        } else {
            0
        };

        let page_size = 50; // Default page size
        let end_index = (start_index + page_size).min(resources.len());
        let page_resources = resources[start_index..end_index].to_vec();

        let next_cursor = if end_index < resources.len() {
            Some(end_index.to_string())
        } else {
            None
        };

        Ok(ResourcesListResponse {
            resources: page_resources,
            next_cursor,
            meta: ResponseMetadata { _meta: None },
        })
    }

    /// Read a specific resource
    pub async fn read_resource(
        &self,
        request: ResourcesReadRequest,
    ) -> Result<ResourcesReadResponse> {
        let handlers = self.handlers.read().await;
        let uri_str = request.uri.to_string();

        if let Some(handler) = handlers.get(&uri_str) {
            let contents = handler.read().await?;
            Ok(ResourcesReadResponse {
                contents,
                meta: ResponseMetadata { _meta: None },
            })
        } else {
            Err(Error::Protocol(
                mocopr_core::error::ProtocolError::ResourceNotFound(uri_str),
            ))
        }
    }

    /// Subscribe to resource updates
    pub async fn subscribe_resource(
        &self,
        request: ResourcesSubscribeRequest,
    ) -> Result<ResourcesSubscribeResponse> {
        let handlers = self.handlers.read().await;
        let uri_str = request.uri.to_string();

        if let Some(handler) = handlers.get(&uri_str) {
            if handler.supports_subscription() {
                handler.subscribe().await?;
                Ok(ResourcesSubscribeResponse {
                    meta: ResponseMetadata { _meta: None },
                })
            } else {
                Err(Error::InvalidRequest(
                    "Resource does not support subscription".to_string(),
                ))
            }
        } else {
            Err(Error::Protocol(
                mocopr_core::error::ProtocolError::ResourceNotFound(uri_str),
            ))
        }
    }

    /// Unsubscribe from resource updates
    pub async fn unsubscribe_resource(
        &self,
        request: ResourcesUnsubscribeRequest,
    ) -> Result<ResourcesUnsubscribeResponse> {
        let handlers = self.handlers.read().await;
        let uri_str = request.uri.to_string();

        if let Some(handler) = handlers.get(&uri_str) {
            handler.unsubscribe().await?;
            Ok(ResourcesUnsubscribeResponse {
                meta: ResponseMetadata { _meta: None },
            })
        } else {
            Err(Error::Protocol(
                mocopr_core::error::ProtocolError::ResourceNotFound(uri_str),
            ))
        }
    }
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for tool handlers
#[derive(Clone)]
pub struct ToolRegistry {
    handlers: Arc<RwLock<HashMap<String, Box<dyn ToolHandler>>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a tool handler
    pub fn register(&mut self, handler: Box<dyn ToolHandler>) {
        let name = futures::executor::block_on(async { handler.tool().await.name });

        futures::executor::block_on(async {
            self.handlers.write().await.insert(name, handler);
        });
    }

    /// List all tools
    pub async fn list_tools(&self, request: ToolsListRequest) -> Result<ToolsListResponse> {
        let handlers = self.handlers.read().await;
        let mut tools = Vec::new();

        for handler in handlers.values() {
            tools.push(handler.tool().await);
        }

        // Apply pagination if cursor is provided
        let start_index = if let Some(cursor) = &request.pagination.cursor {
            cursor.parse::<usize>().unwrap_or(0)
        } else {
            0
        };

        let page_size = 50; // Default page size
        let end_index = (start_index + page_size).min(tools.len());
        let page_tools = tools[start_index..end_index].to_vec();

        let next_cursor = if end_index < tools.len() {
            Some(end_index.to_string())
        } else {
            None
        };

        Ok(ToolsListResponse {
            tools: page_tools,
            next_cursor,
            meta: ResponseMetadata { _meta: None },
        })
    }

    /// Call a specific tool
    pub async fn call_tool(&self, request: ToolsCallRequest) -> Result<ToolsCallResponse> {
        let handlers = self.handlers.read().await;

        if let Some(handler) = handlers.get(&request.name) {
            handler.call(request.arguments).await
        } else {
            Err(Error::Protocol(
                mocopr_core::error::ProtocolError::ToolNotFound(request.name),
            ))
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for prompt handlers
#[derive(Clone)]
pub struct PromptRegistry {
    handlers: Arc<RwLock<HashMap<String, Box<dyn PromptHandler>>>>,
}

impl PromptRegistry {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a prompt handler
    pub fn register(&mut self, handler: Box<dyn PromptHandler>) {
        let name = futures::executor::block_on(async { handler.prompt().await.name });

        futures::executor::block_on(async {
            self.handlers.write().await.insert(name, handler);
        });
    }

    /// List all prompts
    pub async fn list_prompts(&self, request: PromptsListRequest) -> Result<PromptsListResponse> {
        let handlers = self.handlers.read().await;
        let mut prompts = Vec::new();

        for handler in handlers.values() {
            prompts.push(handler.prompt().await);
        }

        // Apply pagination if cursor is provided
        let start_index = if let Some(cursor) = &request.pagination.cursor {
            cursor.parse::<usize>().unwrap_or(0)
        } else {
            0
        };

        let page_size = 50; // Default page size
        let end_index = (start_index + page_size).min(prompts.len());
        let page_prompts = prompts[start_index..end_index].to_vec();

        let next_cursor = if end_index < prompts.len() {
            Some(end_index.to_string())
        } else {
            None
        };

        Ok(PromptsListResponse {
            prompts: page_prompts,
            next_cursor,
            meta: ResponseMetadata { _meta: None },
        })
    }

    /// Get a specific prompt
    pub async fn get_prompt(&self, request: PromptsGetRequest) -> Result<PromptsGetResponse> {
        let handlers = self.handlers.read().await;

        if let Some(handler) = handlers.get(&request.name) {
            handler.generate(request.arguments).await
        } else {
            Err(Error::Protocol(
                mocopr_core::error::ProtocolError::PromptNotFound(request.name),
            ))
        }
    }
}

impl Default for PromptRegistry {
    fn default() -> Self {
        Self::new()
    }
}
