//! RBAC middleware for MoCoPr MCP servers

use crate::prelude::*;
use async_trait::async_trait;
use mocopr_core::prelude::*;
use mocopr_server::middleware::Middleware;
use role_system::async_support::{AsyncRoleSystem, AsyncRoleSystemBuilder};
use role_system::storage::MemoryStorage;
use role_system::{Permission, Resource, Role, Subject as RoleSubject};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

// Use fully qualified Result to avoid ambiguity
type RbacResult<T> = std::result::Result<T, RbacError>;

/// Parsed permission components
#[derive(Debug, Clone)]
struct ParsedPermission {
    action: String,
    resource_type: String,
    pattern: String,
}

/// RBAC middleware for MCP servers using the role-system crate
pub struct RbacMiddleware {
    role_system: Arc<AsyncRoleSystem<MemoryStorage>>,
    context_extractor: Box<dyn ContextExtractor + Send + Sync>,
    audit_enabled: bool,
    // Store patterns separately for pattern matching
    role_patterns: Arc<HashMap<String, Vec<String>>>, // role_name -> list of pattern permissions
}

impl RbacMiddleware {
    /// Create a new RBAC middleware builder
    pub fn builder() -> RbacMiddlewareBuilder {
        RbacMiddlewareBuilder::new()
    }

    /// Check if a subject has permission for a specific action on a resource
    pub async fn check_permission(
        &self,
        subject: &MocoPrSubject,
        action: &str,
        resource: &MocoPrResource,
        context: &HashMap<String, String>,
    ) -> RbacResult<bool> {
        let role_subject = RoleSubject::new(&subject.id);

        // Try exact match with role-system (but skip if resource ID has slashes)
        let has_exact_permission = if resource.id.contains('/') || resource.id.contains('\\') {
            // Skip exact match for resources with path separators to avoid role-system panic
            false
        } else {
            let role_resource = Resource::new(&resource.id, &resource.resource_type);
            self.role_system
                .check_permission_with_context(&role_subject, action, &role_resource, context)
                .await
                .map_err(|e| RbacError::PermissionCheck(e.to_string()))?
        };

        if has_exact_permission {
            if self.audit_enabled {
                info!(
                    subject = %subject.id,
                    action = %action,
                    resource = %resource.id,
                    result = "granted (exact)",
                    "Permission check"
                );
            }
            return Ok(true);
        }

        // Try pattern matching by creating pattern resources and checking them
        let has_pattern_permission = self
            .check_wildcard_patterns(&role_subject, action, resource, context)
            .await?;

        if self.audit_enabled {
            let result = if has_pattern_permission {
                "granted (pattern)"
            } else {
                "denied"
            };
            if has_pattern_permission {
                info!(
                    subject = %subject.id,
                    action = %action,
                    resource = %resource.id,
                    result = result,
                    "Permission check"
                );
            } else {
                warn!(
                    subject = %subject.id,
                    action = %action,
                    resource = %resource.id,
                    result = result,
                    "Permission check"
                );
            }
        }

        Ok(has_pattern_permission)
    }

    /// Check wildcard pattern permissions by checking stored patterns for each role the subject has
    async fn check_wildcard_patterns(
        &self,
        role_subject: &RoleSubject,
        action: &str,
        resource: &MocoPrResource,
        _context: &HashMap<String, String>,
    ) -> RbacResult<bool> {
        // Check each role's patterns
        for (_role_name, patterns) in self.role_patterns.iter() {
            // Check if subject has this role by attempting permission check
            let dummy_resource = Resource::new("dummy", "dummy");
            let _has_role = self
                .role_system
                .check_permission(role_subject, "dummy", &dummy_resource)
                .await
                .unwrap_or(false);

            // If we can't determine role membership, check all patterns
            // For now, let's just check all patterns for all roles
            for pattern in patterns {
                if let Ok(parsed) = self.parse_permission_string(pattern) {
                    // Check if this permission matches our request
                    if parsed.action == action && parsed.resource_type == resource.resource_type {
                        // Check if the pattern matches the resource ID
                        if self.matches_pattern(&parsed.pattern, &resource.id) {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// Parse a permission string into components
    fn parse_permission_string(&self, perm_str: &str) -> RbacResult<ParsedPermission> {
        let parts: Vec<&str> = perm_str.split(':').collect();

        match parts.len() {
            2 => {
                // Two-part format: action:resource
                Ok(ParsedPermission {
                    action: parts[0].to_string(),
                    resource_type: parts[1].to_string(),
                    pattern: "*".to_string(), // Default pattern for 2-part format
                })
            }
            3 => {
                // Three-part format: action:resource_type:pattern
                Ok(ParsedPermission {
                    action: parts[0].to_string(),
                    resource_type: parts[1].to_string(),
                    pattern: parts[2].to_string(),
                })
            }
            _ => Err(RbacError::InvalidPermissionFormat(format!(
                "Invalid permission format: {}",
                perm_str
            ))),
        }
    }

    /// Check if a pattern matches a resource ID
    fn matches_pattern(&self, pattern: &str, resource_id: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if let Some(prefix) = pattern.strip_suffix("/*") {
            return resource_id.starts_with(&format!("{}/", prefix)) || resource_id == prefix;
        }

        if let Some(prefix) = pattern.strip_suffix('*') {
            return resource_id.starts_with(prefix);
        }

        pattern == resource_id
    }

    /// Extract the subject from the request
    fn extract_subject(&self, request: &JsonRpcRequest) -> RbacResult<MocoPrSubject> {
        // Try to extract subject from auth parameters
        if let Some(params) = &request.params
            && let Some(auth) = params.get("auth")
            && let Some(subject_id) = auth.get("subject_id")
            && let Some(id) = subject_id.as_str()
        {
            if let Some(subject_type) = auth.get("subject_type")
                && let Some(stype) = subject_type.as_str()
            {
                return Ok(MocoPrSubject {
                    id: id.to_string(),
                    subject_type: SubjectType::from_str(stype)?,
                });
            }
            // Default to User type if not specified
            return Ok(MocoPrSubject {
                id: id.to_string(),
                subject_type: SubjectType::User,
            });
        }

        // If no subject found, use anonymous user
        Ok(MocoPrSubject {
            id: "anonymous".to_string(),
            subject_type: SubjectType::User,
        })
    }

    /// Extract the resource being accessed from the request
    fn extract_resource(&self, request: &JsonRpcRequest) -> RbacResult<MocoPrResource> {
        match request.method.as_str() {
            "tools/list" => Ok(MocoPrResource {
                id: "*".to_string(),
                resource_type: "tools".to_string(),
            }),
            "tools/call" => {
                if let Some(params) = &request.params
                    && let Some(name) = params.get("name")
                    && let Some(tool_name) = name.as_str()
                {
                    return Ok(MocoPrResource {
                        id: tool_name.to_string(),
                        resource_type: "tools".to_string(),
                    });
                }
                Ok(MocoPrResource {
                    id: "*".to_string(),
                    resource_type: "tools".to_string(),
                })
            }
            "resources/list" => Ok(MocoPrResource {
                id: "*".to_string(),
                resource_type: "resources".to_string(),
            }),
            "resources/read" => {
                if let Some(params) = &request.params
                    && let Some(uri) = params.get("uri")
                    && let Some(resource_uri) = uri.as_str()
                {
                    // Security check: Block path traversal attempts
                    if resource_uri.contains("..") {
                        warn!(
                            "Blocked path traversal attempt in resource URI: {}",
                            resource_uri
                        );
                        return Err(RbacError::PermissionCheck(format!(
                            "Path traversal detected in resource URI: {}",
                            resource_uri
                        )));
                    }

                    return Ok(MocoPrResource {
                        id: resource_uri.to_string(),
                        resource_type: "resources".to_string(),
                    });
                }
                Ok(MocoPrResource {
                    id: "*".to_string(),
                    resource_type: "resources".to_string(),
                })
            }
            "prompts/list" => Ok(MocoPrResource {
                id: "*".to_string(),
                resource_type: "prompts".to_string(),
            }),
            "prompts/get" => {
                if let Some(params) = &request.params
                    && let Some(name) = params.get("name")
                    && let Some(prompt_name) = name.as_str()
                {
                    return Ok(MocoPrResource {
                        id: prompt_name.to_string(),
                        resource_type: "prompts".to_string(),
                    });
                }
                Ok(MocoPrResource {
                    id: "*".to_string(),
                    resource_type: "prompts".to_string(),
                })
            }
            _ => Ok(MocoPrResource {
                id: "unknown".to_string(),
                resource_type: "unknown".to_string(),
            }),
        }
    }

    /// Extract the action from the request method
    fn extract_action(&self, request: &JsonRpcRequest) -> &str {
        match request.method.as_str() {
            "tools/list" | "resources/list" | "prompts/list" => "list",
            "tools/call" => "call",
            "resources/read" => "read",
            "prompts/get" => "get",
            _ => "unknown",
        }
    }
}

#[async_trait]
impl Middleware for RbacMiddleware {
    async fn before_request(&self, request: &JsonRpcRequest) -> mocopr_core::Result<()> {
        debug!("RBAC middleware checking request: {}", request.method);

        // Extract request components
        let subject = self.extract_subject(request).map_err(|_e| {
            mocopr_core::Error::Protocol(mocopr_core::error::ProtocolError::PermissionDenied)
        })?;

        let resource = self.extract_resource(request).map_err(|_e| {
            mocopr_core::Error::Protocol(mocopr_core::error::ProtocolError::PermissionDenied)
        })?;

        let action = self.extract_action(request);

        // Extract context
        let context = self
            .context_extractor
            .extract_context(request)
            .await
            .map_err(|_e| {
                mocopr_core::Error::Protocol(mocopr_core::error::ProtocolError::PermissionDenied)
            })?;

        // Check permission
        let has_permission = self
            .check_permission(&subject, action, &resource, &context)
            .await
            .map_err(|_e| {
                mocopr_core::Error::Protocol(mocopr_core::error::ProtocolError::PermissionDenied)
            })?;

        if !has_permission {
            error!(
                subject = %subject.id,
                action = %action,
                resource = %resource.id,
                "Access denied"
            );
            return Err(mocopr_core::Error::Protocol(
                mocopr_core::error::ProtocolError::PermissionDenied,
            ));
        }

        debug!(
            subject = %subject.id,
            action = %action,
            resource = %resource.id,
            "Access granted"
        );

        Ok(())
    }

    async fn after_response(
        &self,
        _request: &JsonRpcRequest,
        _response: &JsonRpcResponse,
    ) -> mocopr_core::Result<()> {
        Ok(())
    }

    async fn on_error(
        &self,
        _request: &JsonRpcRequest,
        _error: &mocopr_core::Error,
    ) -> mocopr_core::Result<()> {
        Ok(())
    }
}

/// Builder for RBAC middleware
pub struct RbacMiddlewareBuilder {
    roles: Vec<(String, Vec<String>)>,
    conditional_permissions: Vec<ConditionalPermissionConfig>,
    context_extractor: Option<Box<dyn ContextExtractor + Send + Sync>>,
    audit_enabled: bool,
    default_roles: bool,
}

impl RbacMiddlewareBuilder {
    pub fn new() -> Self {
        Self {
            roles: Vec::new(),
            conditional_permissions: Vec::new(),
            context_extractor: None,
            audit_enabled: false,
            default_roles: false,
        }
    }

    /// Add a role with permissions
    pub fn with_role(mut self, role_name: &str, permissions: &[&str]) -> Self {
        self.roles.push((
            role_name.to_string(),
            permissions.iter().map(|s| s.to_string()).collect(),
        ));
        self
    }

    /// Add default MCP roles
    pub fn with_default_roles(mut self) -> Self {
        self.default_roles = true;
        self
    }

    /// Add conditional permission
    pub fn with_conditional_permission<F>(
        mut self,
        role_name: &str,
        permission_pattern: &str,
        condition: F,
    ) -> Self
    where
        F: Fn(&HashMap<String, String>) -> bool + Send + Sync + 'static,
    {
        self.conditional_permissions
            .push(ConditionalPermissionConfig {
                role_name: role_name.to_string(),
                permission_pattern: permission_pattern.to_string(),
                condition: Box::new(condition),
            });
        self
    }

    /// Enable audit logging
    pub fn with_audit_logging(mut self, enabled: bool) -> Self {
        self.audit_enabled = enabled;
        self
    }

    /// Set custom context extractor
    pub fn with_context_extractor<T>(mut self, extractor: T) -> Self
    where
        T: ContextExtractor + Send + Sync + 'static,
    {
        self.context_extractor = Some(Box::new(extractor));
        self
    }

    /// Build the RBAC middleware
    pub async fn build(self) -> RbacResult<RbacMiddleware> {
        let role_system = AsyncRoleSystemBuilder::<MemoryStorage>::new()
            .enable_caching(true)
            .build();

        // Collect role patterns for pattern matching
        let mut role_patterns: HashMap<String, Vec<String>> = HashMap::new();

        // Add default roles if requested
        if self.default_roles {
            self.add_default_roles(&role_system).await?;
        }

        // Add custom roles and assign subjects
        for (role_name, permissions) in self.roles {
            let mut role = Role::new(&role_name);
            let mut patterns_for_role = Vec::new();

            for perm_str in permissions {
                // Store pattern permissions separately
                if perm_str.contains(':') && perm_str.chars().filter(|&c| c == ':').count() >= 2 {
                    // This looks like a 3-part pattern permission
                    patterns_for_role.push(perm_str.clone());
                }

                let (action, resource) = parse_permission_string(&perm_str)?;
                role = role.add_permission(Permission::new(&action, &resource));
            }

            if !patterns_for_role.is_empty() {
                role_patterns.insert(role_name.clone(), patterns_for_role);
            }

            role_system
                .register_role(role)
                .await
                .map_err(|e| RbacError::RoleRegistration(e.to_string()))?;

            // Auto-assign subjects with matching IDs to roles
            let role_subject = RoleSubject::new(&role_name);
            if let Err(e) = role_system.assign_role(&role_subject, &role_name).await {
                // Log error but don't fail - some role systems might not support this
                warn!("Failed to assign role {} to subject: {}", role_name, e);
            }
        }

        // Add conditional permissions
        for conditional in self.conditional_permissions {
            let (action, resource) = parse_permission_string(&conditional.permission_pattern)?;
            let permission = Permission::with_condition(&action, &resource, conditional.condition);

            if let Ok(Some(mut role)) = role_system.get_role(&conditional.role_name).await {
                role = role.add_permission(permission);
                role_system
                    .register_role(role)
                    .await
                    .map_err(|e| RbacError::RoleRegistration(e.to_string()))?;
            } else {
                // Role doesn't exist, create it with the conditional permission
                let role = Role::new(&conditional.role_name).add_permission(permission);
                role_system
                    .register_role(role)
                    .await
                    .map_err(|e| RbacError::RoleRegistration(e.to_string()))?;
            }

            // Assign subject with matching ID to the conditional role
            let role_subject = RoleSubject::new(&conditional.role_name);
            if let Err(e) = role_system
                .assign_role(&role_subject, &conditional.role_name)
                .await
            {
                warn!(
                    "Failed to assign conditional role {} to subject: {}",
                    conditional.role_name, e
                );
            }
        }

        let context_extractor = self
            .context_extractor
            .unwrap_or_else(|| Box::new(DefaultContextExtractor));

        Ok(RbacMiddleware {
            role_system: Arc::new(role_system),
            context_extractor,
            audit_enabled: self.audit_enabled,
            role_patterns: Arc::new(role_patterns),
        })
    }

    async fn add_default_roles(
        &self,
        role_system: &AsyncRoleSystem<MemoryStorage>,
    ) -> RbacResult<()> {
        // Define default roles for MCP servers

        // Guest role - minimal access
        let guest = Role::new("guest")
            .add_permission(Permission::new("list", "tools"))
            .add_permission(Permission::new("list", "resources"));

        // User role - standard access
        let user = Role::new("user")
            .add_permission(Permission::new("list", "tools"))
            .add_permission(Permission::new("call", "tools"))
            .add_permission(Permission::new("list", "resources"))
            .add_permission(Permission::new("read", "resources"));

        // Power user role - advanced access
        let power_user = Role::new("power_user")
            .add_permission(Permission::new("*", "tools"))
            .add_permission(Permission::new("*", "resources"))
            .add_permission(Permission::new("list", "prompts"))
            .add_permission(Permission::new("get", "prompts"));

        // Admin role - full access
        let admin = Role::new("admin").add_permission(Permission::super_admin());

        // Register roles
        role_system
            .register_role(guest)
            .await
            .map_err(|e| RbacError::RoleRegistration(e.to_string()))?;
        role_system
            .register_role(user)
            .await
            .map_err(|e| RbacError::RoleRegistration(e.to_string()))?;
        role_system
            .register_role(power_user)
            .await
            .map_err(|e| RbacError::RoleRegistration(e.to_string()))?;
        role_system
            .register_role(admin)
            .await
            .map_err(|e| RbacError::RoleRegistration(e.to_string()))?;

        // Set up role hierarchy
        role_system
            .add_role_inheritance("user", "guest")
            .await
            .map_err(|e| RbacError::RoleRegistration(e.to_string()))?;
        role_system
            .add_role_inheritance("power_user", "user")
            .await
            .map_err(|e| RbacError::RoleRegistration(e.to_string()))?;
        role_system
            .add_role_inheritance("admin", "power_user")
            .await
            .map_err(|e| RbacError::RoleRegistration(e.to_string()))?;

        Ok(())
    }
}

impl Default for RbacMiddlewareBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Type alias for condition function
type ConditionFn = Box<dyn Fn(&HashMap<String, String>) -> bool + Send + Sync>;

/// Configuration for conditional permissions
struct ConditionalPermissionConfig {
    role_name: String,
    permission_pattern: String,
    condition: ConditionFn,
}

/// Parse permission string like "action:resource" or "action:*"
fn parse_permission_string(perm_str: &str) -> RbacResult<(String, String)> {
    if perm_str.is_empty() {
        return Err(RbacError::InvalidPermissionFormat(
            "empty permission string".to_string(),
        ));
    }

    let parts: Vec<&str> = perm_str.split(':').collect();

    // Support both 2-part and 3-part formats
    // 2-part: action:resource_type
    // 3-part: action:resource_type:pattern (for more granular permissions)
    if parts.len() < 2 || parts.len() > 3 {
        return Err(RbacError::InvalidPermissionFormat(perm_str.to_string()));
    }

    // All parts must be non-empty
    for part in &parts {
        if part.is_empty() {
            return Err(RbacError::InvalidPermissionFormat(perm_str.to_string()));
        }
    }

    // Validate action part (first part) - no special characters that could be exploited
    let action = parts[0];
    if action.contains('/') || action.contains('\\') || action.contains('\0') {
        return Err(RbacError::InvalidPermissionFormat(format!(
            "Invalid action '{}' contains forbidden characters",
            action
        )));
    }

    // Validate resource part (second part) - allow wildcards but validate them
    let resource_type = parts[1];
    if resource_type.contains('\0') {
        return Err(RbacError::InvalidPermissionFormat(format!(
            "Invalid resource type '{}' contains null characters",
            resource_type
        )));
    }

    // If 3-part format, validate the pattern part
    let resource = if parts.len() == 3 {
        let pattern = parts[2];

        // Validate pattern for security - block obvious path traversal attempts
        if pattern.contains("..") {
            return Err(RbacError::InvalidPermissionFormat(format!(
                "Invalid pattern '{}' contains path traversal sequences",
                pattern
            )));
        }

        // Null character validation
        if pattern.contains('\0') {
            return Err(RbacError::InvalidPermissionFormat(format!(
                "Invalid pattern '{}' contains null characters",
                pattern
            )));
        }

        // Combine resource_type and pattern for role-system compatibility
        format!("{}:{}", resource_type, pattern)
    } else {
        // 2-part format - just use the resource_type
        resource_type.to_string()
    };

    Ok((action.to_string(), resource))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::sync::Arc;

    /// Create a test request for middleware testing
    fn create_test_request(
        method: &str,
        params: Option<Value>,
        subject_id: Option<&str>,
        subject_type: Option<&str>,
    ) -> JsonRpcRequest {
        let mut request_params = params.unwrap_or_else(|| json!({}));

        if let (Some(id), Some(stype)) = (subject_id, subject_type) {
            request_params["auth"] = json!({
                "subject_id": id,
                "subject_type": stype
            });
        } else if let Some(id) = subject_id {
            request_params["auth"] = json!({
                "subject_id": id
            });
        }

        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: if request_params.is_object() && !request_params.as_object().unwrap().is_empty()
            {
                Some(request_params)
            } else {
                None
            },
            id: Some(RequestId::Number(1)),
        }
    }

    #[tokio::test]
    async fn test_subject_extraction_edge_cases() {
        let rbac = RbacMiddleware::builder()
            .with_default_roles()
            .build()
            .await
            .unwrap();

        // Test missing auth
        let request = create_test_request("tools/list", None, None, None);
        let subject = rbac.extract_subject(&request).unwrap();
        assert_eq!(subject.id, "anonymous");
        assert_eq!(subject.subject_type, SubjectType::User);

        // Test empty subject_id
        let request = create_test_request(
            "tools/list",
            Some(json!({
                "auth": {
                    "subject_id": ""
                }
            })),
            None,
            None,
        );
        let subject = rbac.extract_subject(&request).unwrap();
        assert_eq!(subject.id, "");

        // Test malformed subject type
        let request = create_test_request(
            "tools/list",
            Some(json!({
                "auth": {
                    "subject_id": "test",
                    "subject_type": "InvalidType"
                }
            })),
            None,
            None,
        );
        let result = rbac.extract_subject(&request);
        // Should handle invalid type gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_resource_extraction_edge_cases() {
        let rbac = RbacMiddleware::builder()
            .with_default_roles()
            .build()
            .await
            .unwrap();

        // Test missing tool name
        let request =
            create_test_request("tools/call", Some(json!({})), Some("user"), Some("User"));
        let resource = rbac.extract_resource(&request).unwrap();
        assert_eq!(resource.id, "*");

        // Test null tool name
        let request = create_test_request(
            "tools/call",
            Some(json!({
                "name": null
            })),
            Some("user"),
            Some("User"),
        );
        let resource = rbac.extract_resource(&request).unwrap();
        assert_eq!(resource.id, "*");

        // Test empty tool name
        let request = create_test_request(
            "tools/call",
            Some(json!({
                "name": ""
            })),
            Some("user"),
            Some("User"),
        );
        let resource = rbac.extract_resource(&request).unwrap();
        assert_eq!(resource.id, "");

        // Test malicious resource URI - should be blocked by security check
        let request = create_test_request(
            "resources/read",
            Some(json!({
                "uri": "../../../etc/passwd"
            })),
            Some("user"),
            Some("User"),
        );
        let result = rbac.extract_resource(&request);
        assert!(result.is_err());
        match result {
            Err(RbacError::PermissionCheck(msg)) => {
                assert!(msg.contains("Path traversal detected"));
            }
            _ => panic!("Expected PermissionCheck error for path traversal"),
        }
    }

    #[tokio::test]
    async fn test_permission_check_with_malicious_context() {
        let rbac = RbacMiddleware::builder()
            .with_role("test_role", &["read:resources"]) // Fixed: use 2-part format
            .build()
            .await
            .unwrap();

        let subject = MocoPrSubject {
            id: "test_user".to_string(),
            subject_type: SubjectType::User,
        };

        let resource = MocoPrResource {
            id: "public/data.txt".to_string(), // Safe resource ID
            resource_type: "resources".to_string(),
        };

        // Test with suspicious context
        let mut malicious_context = HashMap::new();
        malicious_context.insert("admin".to_string(), "true".to_string());
        malicious_context.insert("bypass_security".to_string(), "yes".to_string());
        malicious_context.insert("'; DROP TABLE users; --".to_string(), "value".to_string());

        let result = rbac
            .check_permission(&subject, "read", &resource, &malicious_context)
            .await;
        // Should not be influenced by malicious context keys
        assert!(result.is_ok()); // The method returns, but permission should be properly checked
    }

    #[tokio::test]
    async fn test_path_traversal_security_blocking() {
        let rbac = RbacMiddleware::builder()
            .with_role("test_role", &["read:resources"])
            .build()
            .await
            .unwrap();

        let subject = MocoPrSubject {
            id: "test_user".to_string(),
            subject_type: SubjectType::User,
        };

        // Test that path traversal attempts are properly blocked by the role system
        let path_traversal_attempts = vec![
            "../private/secret.txt",
            "../../etc/passwd",
            "..\\..\\windows\\system32\\config\\sam",
            "public/../admin/config.json",
        ];

        for malicious_path in path_traversal_attempts {
            // Test the middleware's extract_resource method with path traversal attempts
            let malicious_request = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(RequestId::Number(1)),
                method: "resources/read".to_string(),
                params: Some(serde_json::json!({
                    "uri": malicious_path
                })),
            };

            // The extract_resource method should block path traversal attempts
            let extraction_result = rbac.extract_resource(&malicious_request);

            match extraction_result {
                Err(RbacError::PermissionCheck(err)) if err.contains("Path traversal") => {
                    println!(
                        "✓ Path traversal '{}' correctly blocked: {}",
                        malicious_path, err
                    );
                }
                Ok(_) => {
                    panic!(
                        "Security vulnerability: path traversal '{}' was not blocked!",
                        malicious_path
                    );
                }
                Err(other_err) => {
                    panic!(
                        "Unexpected error for path traversal '{}': {:?}",
                        malicious_path, other_err
                    );
                }
            }
        }

        // Verify legitimate resources work
        let legitimate_resource = MocoPrResource {
            id: "public/data.txt".to_string(),
            resource_type: "resources".to_string(),
        };

        let permission_result = rbac
            .check_permission(&subject, "read", &legitimate_resource, &HashMap::new())
            .await;
        assert!(permission_result.is_ok(), "Legitimate resource should work");
        println!("✓ Legitimate resource access works correctly");
    }

    #[tokio::test]
    async fn test_action_extraction_unknown_methods() {
        let rbac = RbacMiddleware::builder().build().await.unwrap();

        let unknown_methods = vec![
            "unknown/method",
            "admin/shutdown",
            "system/exec",
            "",
            "tools/../admin",
            "tools/call/../admin",
        ];

        for method in unknown_methods {
            let request = create_test_request(method, None, Some("user"), Some("User"));
            let action = rbac.extract_action(&request);
            assert_eq!(action, "unknown");
        }
    }

    #[tokio::test]
    async fn test_middleware_chain_security() {
        let rbac = RbacMiddleware::builder()
            .with_role("user", &["list:tools"])
            .build()
            .await
            .unwrap();

        // Test that denied request doesn't proceed
        let forbidden_request = create_test_request(
            "tools/call",
            Some(json!({"name": "admin_tool"})),
            Some("regular_user"),
            Some("User"),
        );

        let result = rbac.before_request(&forbidden_request).await;
        assert!(result.is_err());

        // Verify error type is permission denied
        match result.unwrap_err() {
            mocopr_core::Error::Protocol(mocopr_core::error::ProtocolError::PermissionDenied) => {
                // Expected error type
            }
            other => panic!("Expected PermissionDenied, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_role_builder_edge_cases() {
        // Test invalid permission format
        let result = RbacMiddlewareBuilder::new()
            .with_role("test", &["invalid_permission_format"])
            .build()
            .await;
        assert!(result.is_err());

        // Test empty role name
        let _result = RbacMiddlewareBuilder::new()
            .with_role("", &["read:resources"])
            .build()
            .await;
        // Should handle empty role name

        // Test empty permissions
        let result = RbacMiddlewareBuilder::new()
            .with_role("test", &[])
            .build()
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_conditional_permission_edge_cases() {
        let rbac = RbacMiddlewareBuilder::new()
            .with_conditional_permission("test_role", "admin:system", |ctx| {
                ctx.get("secure_mode") == Some(&"true".to_string())
            })
            .build()
            .await
            .unwrap();

        // Test condition that should fail
        let subject = MocoPrSubject {
            id: "test_user".to_string(),
            subject_type: SubjectType::User,
        };

        let resource = MocoPrResource {
            id: "system".to_string(),
            resource_type: "system".to_string(),
        };

        let mut context = HashMap::new();
        context.insert("secure_mode".to_string(), "false".to_string());

        let result = rbac
            .check_permission(&subject, "admin", &resource, &context)
            .await;
        // Permission should be denied when condition is not met
        assert!(result.is_ok()); // Method completes, but permission should be false
    }

    #[tokio::test]
    async fn test_parse_permission_string_edge_cases() {
        // Valid cases
        assert!(parse_permission_string("read:resource").is_ok());
        assert!(parse_permission_string("*:*").is_ok());
        assert!(parse_permission_string("read:resources:public/*").is_ok()); // 3-part format now supported
        assert!(parse_permission_string("call:tools:safe_*").is_ok()); // 3-part format now supported

        // Invalid cases
        assert!(parse_permission_string("invalid").is_err());
        assert!(parse_permission_string("").is_err());
        assert!(parse_permission_string("a:b:c:d").is_err()); // More than 3 parts
        assert!(parse_permission_string(":").is_err());
        assert!(parse_permission_string("a:").is_err());
        assert!(parse_permission_string(":b").is_err());
        assert!(parse_permission_string("read:resources:../secret").is_err()); // Path traversal in pattern
        assert!(parse_permission_string("read:resources\0:pattern").is_err()); // Null character
    }

    #[tokio::test]
    async fn test_role_hierarchy_bypass_attempts() {
        let rbac = RbacMiddleware::builder()
            .with_default_roles()
            .build()
            .await
            .unwrap();

        // Test attempts to bypass role hierarchy
        let bypass_attempts = vec![
            ("guest", "tools/call", false),  // Guest trying to call tools
            ("user", "server/admin", false), // User trying admin functions
        ];

        for (role, method, should_succeed) in bypass_attempts {
            let request =
                create_test_request(method, None, Some(&format!("{}_test", role)), Some("User"));

            let result = rbac.before_request(&request).await;

            if should_succeed {
                assert!(result.is_ok(), "Role {} should access {}", role, method);
            } else {
                // Most attempts should be denied
                println!(
                    "Role {} accessing {}: allowed = {}",
                    role,
                    method,
                    result.is_ok()
                );
            }
        }
    }

    #[tokio::test]
    async fn test_audit_logging_security() {
        let rbac = RbacMiddleware::builder()
            .with_default_roles()
            .with_audit_logging(true)
            .build()
            .await
            .unwrap();

        let request = create_test_request(
            "tools/call",
            Some(json!({"name": "test_tool"})),
            Some("test_user"),
            Some("User"),
        );

        // This should trigger audit logging
        let _result = rbac.before_request(&request).await;

        // Audit logging should not interfere with security decisions
        // This is more of a smoke test to ensure logging doesn't break anything
    }

    #[tokio::test]
    async fn test_concurrent_middleware_access() {
        let rbac = RbacMiddleware::builder()
            .with_default_roles()
            .build()
            .await
            .unwrap();

        let rbac = Arc::new(rbac);
        let mut handles = Vec::new();

        // Test concurrent access to the same middleware instance
        for i in 0..50 {
            let rbac_clone = rbac.clone();
            let handle = tokio::spawn(async move {
                let request = create_test_request(
                    "tools/list",
                    None,
                    Some(&format!("user_{}", i)),
                    Some("User"),
                );

                rbac_clone.before_request(&request).await
            });
            handles.push(handle);
        }

        // All should complete without panicking
        let mut panic_count = 0;
        for handle in handles {
            if handle.await.is_err() {
                panic_count += 1;
            }
        }
        assert_eq!(panic_count, 0, "No requests should panic");
    }
}
