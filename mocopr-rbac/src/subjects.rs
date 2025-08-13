//! Subject types and representations for MoCoPr RBAC

use crate::error::RbacError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Subject types supported by MoCoPr RBAC
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubjectType {
    /// Human user
    User,
    /// Automated service or application
    Service,
    /// IoT device or system
    Device,
    /// Group of users or entities
    Group,
    /// Custom subject type
    Custom(String),
}

impl std::str::FromStr for SubjectType {
    /// Parse subject type from string
    fn from_str(s: &str) -> Result<Self, RbacError> {
        match s.to_lowercase().as_str() {
            "user" => Ok(SubjectType::User),
            "service" => Ok(SubjectType::Service),
            "device" => Ok(SubjectType::Device),
            "group" => Ok(SubjectType::Group),
            _ => Ok(SubjectType::Custom(s.to_string())),
        }
    }

    type Err = RbacError;
}

impl fmt::Display for SubjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SubjectType::User => write!(f, "user"),
            SubjectType::Service => write!(f, "service"),
            SubjectType::Device => write!(f, "device"),
            SubjectType::Group => write!(f, "group"),
            SubjectType::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// MoCoPr-specific subject representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MocoPrSubject {
    /// Unique identifier for the subject
    pub id: String,
    /// Type of subject
    pub subject_type: SubjectType,
}

impl MocoPrSubject {
    /// Create a new user subject
    pub fn user(id: &str) -> Self {
        Self {
            id: id.to_string(),
            subject_type: SubjectType::User,
        }
    }

    /// Create a new service subject
    pub fn service(id: &str) -> Self {
        Self {
            id: id.to_string(),
            subject_type: SubjectType::Service,
        }
    }

    /// Create a new device subject
    pub fn device(id: &str) -> Self {
        Self {
            id: id.to_string(),
            subject_type: SubjectType::Device,
        }
    }

    /// Create a new group subject
    pub fn group(id: &str) -> Self {
        Self {
            id: id.to_string(),
            subject_type: SubjectType::Group,
        }
    }

    /// Create a custom subject type
    pub fn custom(id: &str, custom_type: &str) -> Self {
        Self {
            id: id.to_string(),
            subject_type: SubjectType::Custom(custom_type.to_string()),
        }
    }
}

impl fmt::Display for MocoPrSubject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.subject_type, self.id)
    }
}
