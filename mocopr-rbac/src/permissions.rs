//! Permission and resource types for MoCoPr RBAC

use serde::{Deserialize, Serialize};
use std::fmt;

/// MCP-specific resource representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MocoPrResource {
    /// Resource identifier (URI, name, pattern, etc.)
    pub id: String,
    /// Resource type (tools, resources, prompts, etc.)
    pub resource_type: String,
}

impl MocoPrResource {
    /// Create a new resource
    pub fn new(id: &str, resource_type: &str) -> Self {
        Self {
            id: id.to_string(),
            resource_type: resource_type.to_string(),
        }
    }

    /// Create a tool resource
    pub fn tool(name: &str) -> Self {
        Self::new(name, "tools")
    }

    /// Create a file resource
    pub fn file_resource(uri: &str) -> Self {
        Self::new(uri, "resources")
    }

    /// Create a prompt resource
    pub fn prompt(name: &str) -> Self {
        Self::new(name, "prompts")
    }

    /// Create a wildcard resource for a type
    pub fn wildcard(resource_type: &str) -> Self {
        Self::new("*", resource_type)
    }
}

impl fmt::Display for MocoPrResource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.resource_type, self.id)
    }
}

/// Common MCP permission patterns
pub struct McpPermissions;

impl McpPermissions {
    /// Tools permissions
    pub const TOOLS_LIST: &'static str = "list:tools";
    pub const TOOLS_CALL: &'static str = "call:tools";
    pub const TOOLS_CALL_ALL: &'static str = "call:tools:*";

    /// Resources permissions  
    pub const RESOURCES_LIST: &'static str = "list:resources";
    pub const RESOURCES_READ: &'static str = "read:resources";
    pub const RESOURCES_READ_ALL: &'static str = "read:resources:*";

    /// Prompts permissions
    pub const PROMPTS_LIST: &'static str = "list:prompts";
    pub const PROMPTS_GET: &'static str = "get:prompts";
    pub const PROMPTS_GET_ALL: &'static str = "get:prompts:*";

    /// Server management permissions
    pub const SERVER_MANAGE: &'static str = "manage:server";
    pub const SERVER_MONITOR: &'static str = "monitor:server";

    /// Admin permissions
    pub const ADMIN_ALL: &'static str = "*:*";
}

/// Permission builder for common MCP patterns
pub struct PermissionBuilder;

impl PermissionBuilder {
    /// Build permission for tool access
    pub fn tool_permission(action: &str, tool_name: &str) -> String {
        format!("{}:tools:{}", action, tool_name)
    }

    /// Build permission for resource access
    pub fn resource_permission(action: &str, resource_uri: &str) -> String {
        format!("{}:resources:{}", action, resource_uri)
    }

    /// Build permission for prompt access
    pub fn prompt_permission(action: &str, prompt_name: &str) -> String {
        format!("{}:prompts:{}", action, prompt_name)
    }

    /// Build wildcard permission for resource type
    pub fn wildcard_permission(action: &str, resource_type: &str) -> String {
        format!("{}:{}:*", action, resource_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_creation() {
        let tool = MocoPrResource::tool("calculator");
        assert_eq!(tool.id, "calculator");
        assert_eq!(tool.resource_type, "tools");

        let file = MocoPrResource::file_resource("file://data/test.txt");
        assert_eq!(file.resource_type, "resources");
    }

    #[test]
    fn test_permission_builder() {
        assert_eq!(
            PermissionBuilder::tool_permission("call", "calculator"),
            "call:tools:calculator"
        );

        assert_eq!(
            PermissionBuilder::wildcard_permission("read", "resources"),
            "read:resources:*"
        );
    }
}
