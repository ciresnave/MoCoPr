//! Prompt-related types and messages

use super::*;

/// Prompt represents a template for generating messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    /// The name of the prompt
    pub name: String,
    /// Optional description of what the prompt does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional list of arguments this prompt accepts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

/// Argument definition for prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    /// The name of the argument
    pub name: String,
    /// Optional description of what this argument is for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this argument is required (defaults to false if not specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// Message in a prompt response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    /// The role of the message sender
    pub role: MessageRole,
    /// The content of the message
    pub content: Content,
}

/// Message roles
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// Message from a user
    User,
    /// Message from an assistant/AI
    Assistant,
    /// System message providing context or instructions
    System,
}

/// Alias for MessageRole for backward compatibility
pub type Role = MessageRole;

/// Request to list available prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsListRequest {
    /// Pagination parameters to control the number of prompts returned
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// Response to list prompts request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsListResponse {
    /// List of available prompts
    pub prompts: Vec<Prompt>,
    /// Cursor for pagination to get the next page of results
    #[serde(rename = "nextCursor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Response metadata
    #[serde(flatten)]
    pub meta: ResponseMetadata,
}

/// Request to get a specific prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsGetRequest {
    /// The name of the prompt to retrieve
    pub name: String,
    /// Optional arguments to pass to the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, String>>,
}

/// Response to get prompt request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsGetResponse {
    /// Optional description of the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The messages that make up the prompt
    pub messages: Vec<PromptMessage>,
    /// Response metadata
    #[serde(flatten)]
    pub meta: ResponseMetadata,
}

/// Alias for PromptsGetResponse for backward compatibility
pub type GetPromptResponse = PromptsGetResponse;

/// Notification that prompts list has changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsListChangedNotification {
    // No additional fields required
}

impl Prompt {
    /// Creates a new prompt with the given name
    ///
    /// # Arguments
    /// * `name` - The name of the prompt
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            arguments: None,
        }
    }

    /// Adds a description to the prompt
    ///
    /// # Arguments
    /// * `description` - Human-readable description of what the prompt does
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Adds arguments to the prompt
    ///
    /// # Arguments
    /// * `arguments` - List of arguments this prompt accepts
    pub fn with_arguments(mut self, arguments: Vec<PromptArgument>) -> Self {
        self.arguments = Some(arguments);
        self
    }
}

impl PromptArgument {
    /// Creates a new prompt argument with the given name
    ///
    /// # Arguments
    /// * `name` - The name of the argument
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            required: None,
        }
    }

    /// Adds a description to the argument
    ///
    /// # Arguments
    /// * `description` - Human-readable description of what this argument is for
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets whether this argument is required
    ///
    /// # Arguments
    /// * `required` - Whether this argument is required
    pub fn required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }
}

impl PromptMessage {
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

impl PromptsListRequest {
    /// Creates a new prompts list request
    ///
    /// # Returns
    /// A new PromptsListRequest instance
    pub fn new() -> Self {
        Self {
            pagination: PaginationParams { cursor: None },
        }
    }

    /// Sets the cursor for pagination
    ///
    /// # Arguments
    /// * `cursor` - The cursor string for pagination
    ///
    /// # Returns
    /// The PromptsListRequest instance with cursor set
    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.pagination.cursor = Some(cursor.into());
        self
    }
}

impl Default for PromptsListRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptsGetRequest {
    /// Creates a new prompts get request
    ///
    /// # Arguments
    /// * `name` - The name of the prompt to retrieve
    ///
    /// # Returns
    /// A new PromptsGetRequest instance
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: None,
        }
    }

    /// Sets the arguments for the prompt
    ///
    /// # Arguments
    /// * `arguments` - A hash map of argument names to values
    ///
    /// # Returns
    /// The PromptsGetRequest instance with arguments set
    pub fn with_arguments(mut self, arguments: HashMap<String, String>) -> Self {
        self.arguments = Some(arguments);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_creation() {
        let prompt = Prompt {
            name: "code_review".to_string(),
            description: Some("Review code for best practices".to_string()),
            arguments: Some(vec![
                PromptArgument {
                    name: "language".to_string(),
                    description: Some("Programming language".to_string()),
                    required: Some(true),
                },
                PromptArgument {
                    name: "code".to_string(),
                    description: Some("Code to review".to_string()),
                    required: Some(true),
                },
            ]),
        };

        assert_eq!(prompt.name, "code_review");
        assert_eq!(
            prompt.description,
            Some("Review code for best practices".to_string())
        );
        assert!(prompt.arguments.is_some());
        assert_eq!(prompt.arguments.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_prompt_argument() {
        let arg = PromptArgument {
            name: "input_text".to_string(),
            description: Some("Text to process".to_string()),
            required: Some(true),
        };

        assert_eq!(arg.name, "input_text");
        assert_eq!(arg.description, Some("Text to process".to_string()));
        assert_eq!(arg.required, Some(true));
    }

    #[test]
    fn test_prompt_serialization() {
        let prompt = Prompt {
            name: "summarize".to_string(),
            description: Some("Summarize text content".to_string()),
            arguments: Some(vec![PromptArgument {
                name: "text".to_string(),
                description: Some("Text to summarize".to_string()),
                required: Some(true),
            }]),
        };

        let serialized = serde_json::to_string(&prompt).unwrap();
        let deserialized: Prompt = serde_json::from_str(&serialized).unwrap();

        assert_eq!(prompt.name, deserialized.name);
        assert_eq!(prompt.description, deserialized.description);
        assert_eq!(
            prompt.arguments.as_ref().unwrap().len(),
            deserialized.arguments.as_ref().unwrap().len()
        );
    }

    #[test]
    fn test_prompt_minimal() {
        let prompt = Prompt {
            name: "simple_prompt".to_string(),
            description: None,
            arguments: None,
        };

        let json_val = serde_json::to_value(&prompt).unwrap();
        assert_eq!(json_val["name"], "simple_prompt");
        assert!(json_val.get("description").is_none());
        assert!(json_val.get("arguments").is_none());
    }

    #[test]
    fn test_prompt_argument_serialization() {
        let arg = PromptArgument {
            name: "test_arg".to_string(),
            description: Some("Test argument".to_string()),
            required: Some(false),
        };

        let serialized = serde_json::to_string(&arg).unwrap();
        let deserialized: PromptArgument = serde_json::from_str(&serialized).unwrap();

        assert_eq!(arg.name, deserialized.name);
        assert_eq!(arg.description, deserialized.description);
        assert_eq!(arg.required, deserialized.required);
    }
}
