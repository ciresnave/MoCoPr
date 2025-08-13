# MoCoPr GitHub Copilot Instructions

This repository contains **MoCoPr** (More Copper), a comprehensive Rust implementation of the Model Context Protocol (MCP) specification. This file provides custom instructions for GitHub Copilot when working on this project.

## Project Structure

- **`mocopr-core/`**: Core MCP types, protocol logic, and transport abstractions
- **`mocopr-server/`**: High-level server API with builder patterns and handler traits
- **`mocopr-client/`**: High-level client API for connecting to MCP servers
- **`mocopr-macros/`**: Procedural macros for reducing boilerplate
- **`examples/`**: Example implementations demonstrating MCP usage

## Coding Guidelines

### 1. Error Handling
- Use `anyhow::Result<T>` for most error returns
- Use `thiserror` for custom error types in core libraries
- Always provide meaningful error messages
- Prefer `?` operator over `.unwrap()` or `.expect()`

### 2. Async Programming
- Use `async/await` throughout the codebase
- Prefer `tokio` for async runtime
- Use `async-trait` for trait definitions with async methods
- Always handle cancellation properly in long-running operations

### 3. Serialization
- Use `serde` with `derive` feature for JSON serialization
- All public types should implement `Serialize` and `Deserialize`
- Use `#[serde(rename_all = "camelCase")]` for JSON compatibility
- Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields

### 4. Protocol Compliance
- Follow the MCP specification exactly (version 2025-06-18)
- All message types must include proper JSON-RPC 2.0 headers
- Support both request/response and notification patterns
- Implement proper capability negotiation

### 5. API Design
- Use builder patterns for complex configurations
- Provide sensible defaults for optional parameters
- Make APIs chainable where appropriate
- Use type-safe patterns to prevent misuse

### 6. Documentation
- Document all public APIs with rustdoc comments
- Include usage examples in documentation
- Document error conditions and panics
- Add module-level documentation explaining purpose

## Common Patterns

### Server Implementation
```rust
use mocopr_server::ServerBuilder;

let server = ServerBuilder::new()
    .name("My Server")
    .version("1.0.0")
    .add_tool(my_tool)
    .add_resource(my_resource)
    .build()?;

server.run_stdio().await?;
```

### Tool Definition
```rust
use mocopr_macros::Tool;

#[derive(Tool)]
#[tool(name = "my_tool", description = "Does something useful")]
struct MyTool;

impl MyTool {
    async fn call(&self, args: Value) -> Result<Value> {
        // Implementation
    }
    
    fn parameters(&self) -> HashMap<String, ToolParameter> {
        // Parameter definitions
    }
}
```

### Resource Definition
```rust
use mocopr_macros::Resource;

#[derive(Resource)]
#[resource(name = "my_resource", description = "Provides data")]
struct MyResource;

impl MyResource {
    async fn read(&self, uri: &str) -> Result<Value> {
        // Implementation
    }
}
```

## Transport Layers

The project supports multiple transport mechanisms:
- **Stdio**: For process-based communication
- **WebSocket**: For real-time web applications
- **HTTP**: For stateless request/response patterns

When implementing transport features:
- Keep transport logic separate from protocol logic
- Use trait abstractions for transport independence
- Handle connection failures gracefully
- Implement proper backoff/retry mechanisms

## Testing

- Write unit tests for all public APIs
- Use `tokio-test` for async test utilities
- Mock external dependencies in tests
- Include integration tests for end-to-end scenarios
- Test error conditions and edge cases

## Performance Considerations

- Use `Arc` and `Rc` for shared ownership
- Prefer `&str` over `String` in function parameters
- Use `Cow<'_, str>` when string might be borrowed or owned
- Avoid unnecessary allocations in hot paths
- Consider using `smallvec` for small collections

## Security

- Validate all inputs from external sources
- Sanitize file paths and prevent directory traversal
- Use secure random generation for IDs
- Implement proper authentication/authorization if needed
- Be cautious with eval-like operations

## MCP-Specific Guidelines

### Message Handling
- Always validate JSON-RPC 2.0 message format
- Handle both named and positional parameters
- Implement proper error codes from JSON-RPC spec
- Support batch requests where applicable

### Capability Negotiation
- Implement the initialization handshake correctly
- Advertise only implemented capabilities
- Respect client capability limitations
- Handle version mismatches gracefully

### Resource Management
- Implement proper URI schemes for resources
- Support both text and binary content types
- Handle large resources efficiently (streaming)
- Implement proper caching strategies

### Tool Execution
- Validate tool parameters against schemas
- Provide clear error messages for invalid inputs
- Support both synchronous and asynchronous tools
- Implement proper timeout handling

## Examples and Demos

When creating examples:
- Make them self-contained and runnable
- Include comprehensive error handling
- Add helpful comments explaining MCP concepts
- Show both simple and advanced usage patterns
- Include realistic use cases

## Dependencies

Prefer these crates for common functionality:
- `anyhow` and `thiserror` for error handling
- `serde` and `serde_json` for serialization
- `tokio` for async runtime
- `tracing` for logging and debugging
- `uuid` for unique identifiers
- `chrono` for time handling
- `url` for URL parsing

## Contribution Guidelines

When implementing new features:
1. Start with the core types in `mocopr-core`
2. Add server-side support in `mocopr-server`
3. Add client-side support in `mocopr-client`
4. Consider adding convenience macros in `mocopr-macros`
5. Create examples demonstrating the feature
6. Update documentation and README

## Common Gotchas

- Remember that MCP uses JSON-RPC 2.0, not REST
- All method names should be in snake_case (JSON), not camelCase
- Resources use URI schemes, not simple paths
- Tools and prompts have different parameter patterns
- Transport layer handles framing, protocol layer handles content
- Always handle the initialization handshake before other operations

## IDE Integration

This project is designed to work well with:
- Rust Analyzer for IDE support
- Clippy for linting
- rustfmt for code formatting
- cargo-doc for documentation generation

Use `cargo clippy` regularly and fix all warnings before committing.
