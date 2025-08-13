//! Roots-related types (client capabilities)

use super::*;

/// Root represents a filesystem or URI boundary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Root {
    /// The URI of the root
    pub uri: Url,
    /// Optional name for the root
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Request to list roots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsListRequest {
    // No additional fields required
}

/// Response to list roots request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsListResponse {
    /// List of available roots
    pub roots: Vec<Root>,
    /// Response metadata
    #[serde(flatten)]
    pub meta: ResponseMetadata,
}

/// Notification that roots list has changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsListChangedNotification {
    // No additional fields required
}

impl Root {
    /// Creates a new root with the given URI
    ///
    /// # Arguments
    /// * `uri` - The URI of the root
    pub fn new(uri: Url) -> Self {
        Self { uri, name: None }
    }

    /// Sets the name of the root
    ///
    /// # Arguments
    /// * `name` - The name to set
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

impl RootsListRequest {
    /// Creates a new roots list request
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for RootsListRequest {
    fn default() -> Self {
        Self::new()
    }
}
