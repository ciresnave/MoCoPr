//! Sampling-related types (client capabilities)

use super::*;

/// Request for the client to create messages using sampling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    /// The message history to use for generating a new message
    pub messages: Vec<SamplingMessage>,
    /// Maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// Sampling temperature (higher values = more random)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Sequences that will cause generation to stop when encountered
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// System prompt to use for message generation
    #[serde(rename = "systemPrompt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Whether to include context from other servers
    #[serde(rename = "includeContext")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_context: Option<IncludeContext>,
    /// Metadata for the request
    #[serde(flatten)]
    pub metadata: ResponseMetadata,
}

/// Response from sampling/createMessage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessageResponse {
    /// The generated content
    pub content: Content,
    /// The model that generated the response
    pub model: String,
    /// The reason why generation stopped
    #[serde(rename = "stopReason")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
    /// The role of the message (usually assistant)
    pub role: MessageRole,
    /// Metadata for the response
    #[serde(flatten)]
    pub meta: ResponseMetadata,
}

/// Message for sampling requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingMessage {
    /// The role of the message (user, assistant, or system)
    pub role: MessageRole,
    /// The content of the message
    pub content: Content,
}

/// Context inclusion options for sampling
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IncludeContext {
    /// Don't include any additional context
    None,
    /// Include context only from this server
    ThisServer,
    /// Include context from all connected servers
    AllServers,
}

/// Reason sampling stopped
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Natural end of turn in the conversation
    EndTurn,
    /// One of the configured stop sequences was encountered
    StopSequence,
    /// Maximum token length was reached
    MaxTokens,
    /// The model requested to use a tool
    ToolUse,
}

impl SamplingMessage {
    /// Creates a new user message
    ///
    /// # Arguments
    /// * `content` - The content of the message
    pub fn user(content: impl Into<Content>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    /// Creates a new assistant message
    ///
    /// # Arguments
    /// * `content` - The content of the message
    pub fn assistant(content: impl Into<Content>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }

    /// Creates a new system message
    ///
    /// # Arguments
    /// * `content` - The content of the message
    pub fn system(content: impl Into<Content>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }
}

impl CreateMessageRequest {
    /// Creates a new create message request
    ///
    /// # Arguments
    /// * `messages` - The conversation history to use for generating the next message
    pub fn new(messages: Vec<SamplingMessage>) -> Self {
        Self {
            messages,
            max_tokens: None,
            temperature: None,
            stop_sequences: None,
            system_prompt: None,
            include_context: None,
            metadata: ResponseMetadata { _meta: None },
        }
    }

    /// Sets the maximum number of tokens to generate
    ///
    /// # Arguments
    /// * `max_tokens` - The maximum number of tokens to generate
    pub fn with_max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Sets the sampling temperature
    ///
    /// # Arguments
    /// * `temperature` - The sampling temperature (higher values = more random)
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Sets the stop sequences that will halt generation
    ///
    /// # Arguments
    /// * `stop_sequences` - A list of strings that will cause generation to stop
    pub fn with_stop_sequences(mut self, stop_sequences: Vec<String>) -> Self {
        self.stop_sequences = Some(stop_sequences);
        self
    }

    /// Sets the system prompt for the request
    ///
    /// # Arguments
    /// * `system_prompt` - The system prompt to use for generation
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    /// Sets the context inclusion mode
    ///
    /// # Arguments
    /// * `include_context` - How to include context from other servers
    pub fn with_include_context(mut self, include_context: IncludeContext) -> Self {
        self.include_context = Some(include_context);
        self
    }
}
