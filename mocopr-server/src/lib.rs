//! # MoCoPr Server
//!
//! A comprehensive and developer-friendly MCP server implementation in Rust.
//!
//! This crate provides high-level abstractions for building MCP servers with
//! support for resources, tools, prompts, and other MCP features.
//!
//! ## Quick Start
//!
//! ```rust
//! use mocopr_server::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let server = McpServer::builder()
//!         .with_info("My MCP Server", "1.0.0")
//!         .with_resources()
//!         .with_tools()
//!         .build()?;
//!
//!     server.run_stdio().await?;
//!     Ok(())
//! }
//! ```

pub mod builder;
pub mod handlers;
pub mod middleware;
pub mod registry;
pub mod server;

pub use builder::*;
pub use handlers::*;
pub use registry::*;
pub use server::*;

/// Common imports for MCP server development
pub mod prelude {
    pub use crate::builder::*;
    pub use crate::handlers::*;
    pub use crate::registry::*;
    pub use crate::server::*;
    pub use mocopr_core::prelude::*;
    pub use mocopr_macros::*;

    pub use async_trait::async_trait;
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::{Value, json};
    pub use tokio;
    pub use url::Url;
}
