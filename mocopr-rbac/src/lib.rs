//! Role-Based Access Control (RBAC) integration for MoCoPr MCP servers
//!
//! This crate provides seamless integration between MoCoPr and the role-system crate,
//! enabling fine-grained authorization for MCP protocol operations.
//!
//! # Features
//!
//! - **Hierarchical Roles**: Support for role inheritance (admin > power_user > user)
//! - **Fine-grained Permissions**: Control access to specific tools, resources, and prompts
//! - **Conditional Permissions**: Context-based access control (time, location, etc.)
//! - **Multiple Subject Types**: Support for users, services, devices, and groups
//! - **Async Support**: Full async/await compatibility with MoCoPr
//! - **Audit Logging**: Comprehensive security event logging
//! - **Persistence**: Optional role/permission persistence
//!
//! # Quick Start
//!
//! ```rust
//! use mocopr_rbac::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create RBAC middleware with predefined roles
//!     let rbac = RbacMiddleware::builder()
//!         .with_default_roles()
//!         .with_audit_logging(true)
//!         .build().await?;
//!
//!     println!("RBAC middleware created successfully");
//!     Ok(())
//! }
//! ```
//!
//! # Custom Role Configuration
//!
//! ```rust
//! use mocopr_rbac::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let rbac = RbacMiddleware::builder()
//!         .with_role("admin", &[
//!             "tools:*",
//!             "resources:*",
//!             "prompts:*",
//!             "server:manage"
//!         ])
//!         .with_role("user", &[
//!             "tools:read",
//!             "tools:call:safe/*",
//!             "resources:read:public/*"
//!         ])
//!         .with_conditional_permission(
//!             "power_user",
//!             "tools:call:admin/*",
//!             |context| context.get("verified") == Some(&"true".to_string())
//!         )
//!         .build().await?;
//!
//!     println!("Custom RBAC roles configured successfully");
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod context;
pub mod error;
pub mod middleware;
pub mod permissions;
pub mod subjects;

pub mod prelude {
    //! Common imports for MoCoPr RBAC

    pub use crate::config::*;
    pub use crate::context::*;
    pub use crate::error::*;
    pub use crate::middleware::RbacMiddleware;
    pub use crate::permissions::*;
    pub use crate::subjects::*;

    // Re-export key role-system types
    pub use role_system::{Permission, Resource, Role, Subject as RoleSubject};

    // Common Result type
    pub type Result<T> = std::result::Result<T, RbacError>;
}

// Re-export major components at crate level
pub use error::RbacError;
pub use middleware::RbacMiddleware;
pub use prelude::Result;
