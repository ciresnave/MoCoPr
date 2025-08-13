//! Configuration types for MoCoPr RBAC

use serde::{Deserialize, Serialize};

/// RBAC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacConfig {
    /// Default roles to create
    pub default_roles: bool,
    /// Enable audit logging
    pub audit_enabled: bool,
    /// Cache settings
    pub cache_config: CacheConfig,
    /// Role definitions
    pub roles: Vec<RoleConfig>,
    /// Subject role assignments
    pub assignments: Vec<AssignmentConfig>,
}

impl Default for RbacConfig {
    fn default() -> Self {
        Self {
            default_roles: true,
            audit_enabled: true,
            cache_config: CacheConfig::default(),
            roles: Vec::new(),
            assignments: Vec::new(),
        }
    }
}

/// Cache configuration for role system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable permission caching
    pub enabled: bool,
    /// Cache TTL in seconds
    pub ttl_seconds: u64,
    /// Maximum cache entries
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_seconds: 300, // 5 minutes
            max_entries: 10000,
        }
    }
}

/// Role configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    /// Role name
    pub name: String,
    /// Role description
    pub description: Option<String>,
    /// Static permissions
    pub permissions: Vec<String>,
    /// Conditional permissions
    pub conditional_permissions: Vec<ConditionalPermissionConfig>,
    /// Roles this role inherits from
    pub inherits_from: Vec<String>,
}

/// Conditional permission configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalPermissionConfig {
    /// Permission pattern (e.g., "call:tools:admin/*")
    pub permission: String,
    /// JavaScript-like condition expression
    pub condition: String,
    /// Description of the condition
    pub description: Option<String>,
}

/// Subject role assignment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentConfig {
    /// Subject identifier
    pub subject_id: String,
    /// Subject type
    pub subject_type: String,
    /// Assigned roles
    pub roles: Vec<String>,
    /// Temporary elevation (optional)
    pub elevation: Option<ElevationConfig>,
}

/// Temporary role elevation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevationConfig {
    /// Additional roles for temporary elevation
    pub roles: Vec<String>,
    /// Duration in seconds
    pub duration_seconds: u64,
    /// Justification for elevation
    pub justification: Option<String>,
}

impl RbacConfig {
    /// Load configuration from file
    pub fn from_file(path: &str) -> std::result::Result<Self, crate::error::RbacError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::error::RbacError::Configuration(format!("Failed to read config file: {}", e))
        })?;

        let config: RbacConfig = serde_json::from_str(&content).map_err(|e| {
            crate::error::RbacError::Configuration(format!("Failed to parse config: {}", e))
        })?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn to_file(&self, path: &str) -> std::result::Result<(), crate::error::RbacError> {
        let content = serde_json::to_string_pretty(self).map_err(|e| {
            crate::error::RbacError::Configuration(format!("Failed to serialize config: {}", e))
        })?;

        std::fs::write(path, content).map_err(|e| {
            crate::error::RbacError::Configuration(format!("Failed to write config file: {}", e))
        })?;

        Ok(())
    }

    /// Create a basic configuration for development
    pub fn development() -> Self {
        Self {
            default_roles: true,
            audit_enabled: false, // Disabled for development
            cache_config: CacheConfig {
                enabled: false, // Disabled for development
                ttl_seconds: 60,
                max_entries: 100,
            },
            roles: vec![RoleConfig {
                name: "dev".to_string(),
                description: Some("Development role with elevated access".to_string()),
                permissions: vec![
                    "list:tools".to_string(),
                    "call:tools".to_string(),
                    "read:resources".to_string(),
                    "list:prompts".to_string(),
                ],
                conditional_permissions: Vec::new(),
                inherits_from: Vec::new(),
            }],
            assignments: vec![AssignmentConfig {
                subject_id: "developer".to_string(),
                subject_type: "user".to_string(),
                roles: vec!["dev".to_string()],
                elevation: None,
            }],
        }
    }

    /// Create a production configuration template
    pub fn production_template() -> Self {
        Self {
            default_roles: true,
            audit_enabled: true,
            cache_config: CacheConfig::default(),
            roles: vec![
                RoleConfig {
                    name: "api_client".to_string(),
                    description: Some("Standard API client role".to_string()),
                    permissions: vec![
                        "list:tools".to_string(),
                        "call:tools:safe/*".to_string(),
                        "read:resources:public/*".to_string(),
                    ],
                    conditional_permissions: vec![ConditionalPermissionConfig {
                        permission: "call:tools:admin/*".to_string(),
                        condition:
                            "context.business_hours == 'true' && context.trust_level == 'high'"
                                .to_string(),
                        description: Some(
                            "Admin tools only during business hours from trusted clients"
                                .to_string(),
                        ),
                    }],
                    inherits_from: Vec::new(),
                },
                RoleConfig {
                    name: "service_account".to_string(),
                    description: Some("Automated service account role".to_string()),
                    permissions: vec![
                        "call:tools:automation/*".to_string(),
                        "read:resources:data/*".to_string(),
                    ],
                    conditional_permissions: Vec::new(),
                    inherits_from: vec!["api_client".to_string()],
                },
            ],
            assignments: Vec::new(), // To be filled in production
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> std::result::Result<(), crate::error::RbacError> {
        // Check for duplicate role names
        let mut role_names = std::collections::HashSet::new();
        for role in &self.roles {
            if !role_names.insert(&role.name) {
                return Err(crate::error::RbacError::Configuration(format!(
                    "Duplicate role name: {}",
                    role.name
                )));
            }
        }

        // Check that inherited roles exist
        for role in &self.roles {
            for inherited in &role.inherits_from {
                if !role_names.contains(inherited) && !self.is_default_role(inherited) {
                    return Err(crate::error::RbacError::Configuration(format!(
                        "Role '{}' inherits from non-existent role '{}'",
                        role.name, inherited
                    )));
                }
            }
        }

        // Check that assigned roles exist
        for assignment in &self.assignments {
            for role_name in &assignment.roles {
                if !role_names.contains(role_name) && !self.is_default_role(role_name) {
                    return Err(crate::error::RbacError::Configuration(format!(
                        "Assignment for '{}' references non-existent role '{}'",
                        assignment.subject_id, role_name
                    )));
                }
            }
        }

        Ok(())
    }

    fn is_default_role(&self, role_name: &str) -> bool {
        if !self.default_roles {
            return false;
        }
        matches!(role_name, "guest" | "user" | "power_user" | "admin")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_serialization() {
        let config = RbacConfig::development();
        let json = serde_json::to_string_pretty(&config).unwrap();
        println!("Development config JSON:\n{}", json);

        let parsed: RbacConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.default_roles, config.default_roles);
        assert_eq!(parsed.roles.len(), config.roles.len());
    }

    #[test]
    fn test_config_file_operations() {
        let config = RbacConfig::production_template();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        // Save config
        config.to_file(path).unwrap();

        // Load config
        let loaded = RbacConfig::from_file(path).unwrap();
        assert_eq!(loaded.default_roles, config.default_roles);
        assert_eq!(loaded.roles.len(), config.roles.len());
    }

    #[test]
    fn test_config_validation() {
        let mut config = RbacConfig::default();

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Add duplicate role names
        config.roles.push(RoleConfig {
            name: "duplicate".to_string(),
            description: None,
            permissions: Vec::new(),
            conditional_permissions: Vec::new(),
            inherits_from: Vec::new(),
        });
        config.roles.push(RoleConfig {
            name: "duplicate".to_string(),
            description: None,
            permissions: Vec::new(),
            conditional_permissions: Vec::new(),
            inherits_from: Vec::new(),
        });

        // Should fail validation
        assert!(config.validate().is_err());
    }
}
