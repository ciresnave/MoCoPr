//! Handler traits and implementations for MCP server features

use async_trait::async_trait;
use mocopr_core::prelude::*;
use std::collections::HashMap;

/// Trait for handling resource operations
#[async_trait]
pub trait ResourceHandler: Send + Sync {
    /// Get the resource information
    async fn resource(&self) -> Resource;

    /// Read the resource content
    async fn read(&self) -> Result<Vec<ResourceContent>>;

    /// Check if the resource supports subscriptions
    fn supports_subscription(&self) -> bool {
        false
    }

    /// Subscribe to resource updates
    async fn subscribe(&self) -> Result<()> {
        Err(Error::MethodNotFound("subscribe".to_string()))
    }

    /// Unsubscribe from resource updates
    async fn unsubscribe(&self) -> Result<()> {
        Err(Error::MethodNotFound("unsubscribe".to_string()))
    }
}

/// Trait for handling tool operations
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Get the tool information
    async fn tool(&self) -> Tool;

    /// Execute the tool with given arguments
    async fn call(&self, arguments: Option<serde_json::Value>) -> Result<ToolsCallResponse>;
}

/// Trait for handling prompt operations
#[async_trait]
pub trait PromptHandler: Send + Sync {
    /// Get the prompt information
    async fn prompt(&self) -> Prompt;

    /// Generate the prompt with given arguments
    async fn generate(
        &self,
        arguments: Option<HashMap<String, String>>,
    ) -> Result<PromptsGetResponse>;
}

/// File-based resource handler
pub struct FileResourceHandler {
    uri: url::Url,
    name: String,
    description: Option<String>,
    mime_type: Option<String>,
    file_path: std::path::PathBuf,
}

impl FileResourceHandler {
    pub fn new(
        uri: url::Url,
        name: impl Into<String>,
        file_path: impl Into<std::path::PathBuf>,
    ) -> Self {
        Self {
            uri,
            name: name.into(),
            description: None,
            mime_type: None,
            file_path: file_path.into(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }
}

#[async_trait]
impl ResourceHandler for FileResourceHandler {
    async fn resource(&self) -> Resource {
        Resource {
            uri: self.uri.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            mime_type: self.mime_type.clone(),
            annotations: None,
        }
    }

    async fn read(&self) -> Result<Vec<ResourceContent>> {
        let content = tokio::fs::read_to_string(&self.file_path)
            .await
            .map_err(|e| {
                Error::Protocol(mocopr_core::error::ProtocolError::ResourceNotFound(
                    format!("Failed to read file: {}", e),
                ))
            })?;

        let text_content = TextContent::new(content);
        let resource_content = ResourceContent {
            uri: self.uri.clone(),
            mime_type: self.mime_type.clone(),
            contents: vec![Content::Text(text_content)],
        };

        Ok(vec![resource_content])
    }
}

/// Simple function-based tool handler
pub struct FunctionToolHandler {
    tool_info: Tool,
    handler: Box<dyn Fn(Option<serde_json::Value>) -> Result<ToolsCallResponse> + Send + Sync>,
}

impl FunctionToolHandler {
    pub fn new<F>(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
        handler: F,
    ) -> Self
    where
        F: Fn(Option<serde_json::Value>) -> Result<ToolsCallResponse> + Send + Sync + 'static,
    {
        let tool_info = Tool::new(name, input_schema).with_description(description);

        Self {
            tool_info,
            handler: Box::new(handler),
        }
    }
}

#[async_trait]
impl ToolHandler for FunctionToolHandler {
    async fn tool(&self) -> Tool {
        self.tool_info.clone()
    }

    async fn call(&self, arguments: Option<serde_json::Value>) -> Result<ToolsCallResponse> {
        (self.handler)(arguments)
    }
}

/// Template-based prompt handler
pub struct TemplatePromptHandler {
    prompt_info: Prompt,
    template: String,
}

impl TemplatePromptHandler {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        template: impl Into<String>,
        arguments: Vec<PromptArgument>,
    ) -> Self {
        let prompt_info = Prompt::new(name)
            .with_description(description)
            .with_arguments(arguments);

        Self {
            prompt_info,
            template: template.into(),
        }
    }
}

#[async_trait]
impl PromptHandler for TemplatePromptHandler {
    async fn prompt(&self) -> Prompt {
        self.prompt_info.clone()
    }

    async fn generate(
        &self,
        arguments: Option<HashMap<String, String>>,
    ) -> Result<PromptsGetResponse> {
        let mut content = self.template.clone();

        if let Some(args) = arguments {
            for (key, value) in args {
                let placeholder = format!("{{{}}}", key);
                content = content.replace(&placeholder, &value);
            }
        }

        let message = PromptMessage::user(content);

        Ok(PromptsGetResponse {
            description: self.prompt_info.description.clone(),
            messages: vec![message],
            meta: ResponseMetadata { _meta: None },
        })
    }
}

/// Macro for creating simple tool handlers
#[macro_export]
macro_rules! tool_handler {
    (
        name: $name:expr,
        description: $description:expr,
        schema: $schema:expr,
        handler: $handler:expr
    ) => {
        $crate::handlers::FunctionToolHandler::new($name, $description, $schema, $handler)
    };
}

/// Macro for creating file resource handlers
#[macro_export]
macro_rules! file_resource {
    (
        uri: $uri:expr,
        name: $name:expr,
        path: $path:expr
    ) => {
        $crate::handlers::FileResourceHandler::new($uri.parse().unwrap(), $name, $path)
    };
    (
        uri: $uri:expr,
        name: $name:expr,
        path: $path:expr,
        description: $description:expr
    ) => {
        $crate::handlers::FileResourceHandler::new($uri.parse().unwrap(), $name, $path)
            .with_description($description)
    };
    (
        uri: $uri:expr,
        name: $name:expr,
        path: $path:expr,
        description: $description:expr,
        mime_type: $mime_type:expr
    ) => {
        $crate::handlers::FileResourceHandler::new($uri.parse().unwrap(), $name, $path)
            .with_description($description)
            .with_mime_type($mime_type)
    };
}

/// Macro for creating template prompt handlers
#[macro_export]
macro_rules! template_prompt {
    (
        name: $name:expr,
        description: $description:expr,
        template: $template:expr,
        arguments: [ $( $arg:expr ),* ]
    ) => {
        $crate::handlers::TemplatePromptHandler::new(
            $name,
            $description,
            $template,
            vec![ $( $arg ),* ],
        )
    };
}
