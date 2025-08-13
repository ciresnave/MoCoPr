//! # MoCoPr - Model Context Protocol for Rust
//!
//! A comprehensive Rust implementation of the Model Context Protocol (MCP).
//!
//! This crate re-exports the core functionality from the constituent crates:
//! - `mocopr-core`: Core types and protocol implementation
//! - `mocopr-server`: High-level server implementation
//! - `mocopr-client`: High-level client implementation
//! - `mocopr-macros`: Procedural macros for easier development

pub use mocopr_client as client;
pub use mocopr_core as core;
pub use mocopr_server as server;

/// Re-export commonly used items
pub mod prelude {
    pub use crate::client::prelude::*;
    #[allow(unused_imports)]
    pub use crate::core::prelude::*;
    pub use crate::server::prelude::*;
}
