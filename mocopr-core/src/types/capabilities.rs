//! Capability negotiation types

use super::*;

/// Client capabilities advertised during initialization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientCapabilities {
    /// Experimental capabilities as arbitrary key-value pairs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, serde_json::Value>>,
    /// Client sampling capability for generating messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
    /// Client roots capability for filesystem-like access
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,
}

/// Server capabilities advertised during initialization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerCapabilities {
    /// Experimental capabilities as arbitrary key-value pairs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, serde_json::Value>>,
    /// Server logging capability for controlling log output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapability>,
    /// Server prompts capability for managing prompt templates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
    /// Server resources capability for managing content resources
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    /// Server tools capability for exposing callable tools
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
}

/// Sampling capability (client-side)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingCapability {
    // Currently no additional fields defined
}

/// Roots capability (client-side)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsCapability {
    /// Whether the client supports receiving roots list changed notifications
    #[serde(rename = "listChanged")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Logging capability (server-side)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingCapability {
    // Currently no additional fields defined
}

/// Prompts capability (server-side)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    /// Whether the server supports sending prompts list changed notifications
    #[serde(rename = "listChanged")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Resources capability (server-side)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    /// Whether the server supports sending resources list changed notifications
    #[serde(rename = "listChanged")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
    /// Whether the server supports resource subscriptions for updates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
}

/// Tools capability (server-side)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    /// Whether the server supports sending tools list changed notifications
    #[serde(rename = "listChanged")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

impl ClientCapabilities {
    /// Creates a new default client capabilities instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable the client sampling capability
    pub fn with_sampling(mut self) -> Self {
        self.sampling = Some(SamplingCapability {});
        self
    }

    /// Enable the client roots capability
    ///
    /// # Arguments
    /// * `list_changed` - Whether to receive notifications when the roots list changes
    pub fn with_roots(mut self, list_changed: bool) -> Self {
        self.roots = Some(RootsCapability {
            list_changed: Some(list_changed),
        });
        self
    }

    /// Add an experimental capability
    ///
    /// # Arguments
    /// * `key` - The name of the experimental capability
    /// * `value` - The value for the experimental capability
    pub fn with_experimental(mut self, key: String, value: serde_json::Value) -> Self {
        self.experimental
            .get_or_insert_with(HashMap::new)
            .insert(key, value);
        self
    }
}

impl ServerCapabilities {
    /// Creates a new default server capabilities instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable the server logging capability
    pub fn with_logging(mut self) -> Self {
        self.logging = Some(LoggingCapability {});
        self
    }

    /// Enable the server prompts capability
    ///
    /// # Arguments
    /// * `list_changed` - Whether to send notifications when the prompts list changes
    pub fn with_prompts(mut self, list_changed: bool) -> Self {
        self.prompts = Some(PromptsCapability {
            list_changed: Some(list_changed),
        });
        self
    }

    /// Enable the server resources capability
    ///
    /// # Arguments
    /// * `list_changed` - Whether to send notifications when the resources list changes
    /// * `subscribe` - Whether to support resource subscriptions for updates
    pub fn with_resources(mut self, list_changed: bool, subscribe: bool) -> Self {
        self.resources = Some(ResourcesCapability {
            list_changed: Some(list_changed),
            subscribe: Some(subscribe),
        });
        self
    }

    /// Enable the server tools capability
    ///
    /// # Arguments
    /// * `list_changed` - Whether to send notifications when the tools list changes
    pub fn with_tools(mut self, list_changed: bool) -> Self {
        self.tools = Some(ToolsCapability {
            list_changed: Some(list_changed),
        });
        self
    }

    /// Add an experimental capability
    ///
    /// # Arguments
    /// * `key` - The name of the experimental capability
    /// * `value` - The value for the experimental capability
    pub fn with_experimental(mut self, key: String, value: serde_json::Value) -> Self {
        self.experimental
            .get_or_insert_with(HashMap::new)
            .insert(key, value);
        self
    }
}
