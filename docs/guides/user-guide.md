# MoCoPr User Guide

Welcome to the MoCoPr User Guide! This guide provides a comprehensive overview of the MoCoPr library and its features. Whether you're building a simple MCP server or a complex, production-ready application, this guide has you covered.

## Table of Contents

*   **1. Introduction**
    *   What is MoCoPr?
    *   Core Concepts
*   **2. Getting Started**
    *   Installation
    *   Building Your First Server
*   **3. Core Features**
    *   Resources
    *   Tools
    *   Prompts
*   **4. Advanced Features**
    *   Middleware
    *   Transports
    *   Monitoring
*   **5. Best Practices**
    *   Error Handling
    *   Security
    *   Performance Tuning
*   **6. API Reference**
    *   `mocopr-core`
    *   `mocopr-server`
    *   `mocopr-client`
    *   `mocopr-macros`

## 1. Introduction

### What is MoCoPr?

MoCoPr is a Rust implementation of the Model Context Protocol (MCP), a standard for communication between AI models and their surrounding environment. It provides a flexible and extensible framework for building robust and high-performance MCP servers and clients.

### Core Concepts

*   **Server:** An MCP server provides resources, tools, and prompts to a client.
*   **Client:** An MCP client connects to a server and consumes its resources, tools, and prompts.
*   **Resource:** A piece of data or content that a server can provide (e.g., a file, a database record).
*   **Tool:** A function or operation that a server can execute (e.g., a calculator, a web search).
*   **Prompt:** A template for generating or modifying content.
*   **Transport:** The underlying communication mechanism (e.g., stdio, WebSocket, HTTP).
*   **Middleware:** A component that can process requests and responses before they reach the handler.

## 2. Getting Started

### Installation

To get started with MoCoPr, add the following to your `Cargo.toml` file:

```toml
[dependencies]
mocopr = "0.1.0"
```

### Building Your First Server

Here's a simple example of how to build an MCP server with MoCoPr:

```rust
use mocopr::prelude::*;

#[mocopr::main]
async fn main() -> anyhow::Result<()> {
    let server = mcp_server! {
        name: "My First Server",
        version: "1.0.0",
    }
    .build()?;

    server.run().await?;

    Ok(())
}
```

This example creates a simple server with no resources, tools, or prompts. In the following sections, you'll learn how to add these features to your server.
