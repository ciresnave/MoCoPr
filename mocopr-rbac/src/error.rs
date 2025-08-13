//! Error types for MoCoPr RBAC

use thiserror::Error;

/// Errors that can occur in RBAC operations
#[derive(Error, Debug)]
pub enum RbacError {
    #[error("Permission check failed: {0}")]
    PermissionCheck(String),

    #[error("Role registration failed: {0}")]
    RoleRegistration(String),

    #[error("Invalid subject type: {0}")]
    InvalidSubjectType(String),

    #[error("Invalid permission format: {0}")]
    InvalidPermissionFormat(String),

    #[error("Context extraction failed: {0}")]
    ContextExtraction(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Role system error: {0}")]
    RoleSystem(String),
}
