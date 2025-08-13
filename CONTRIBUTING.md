# CONTRIBUTING.md

Thank you for your interest in contributing to MoCoPr! This guide will help you get started with contributing to the project.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Contributing Guidelines](#contributing-guidelines)
- [Code Style and Standards](#code-style-and-standards)
- [Testing](#testing)
- [Documentation](#documentation)
- [Pull Request Process](#pull-request-process)
- [Issue Reporting](#issue-reporting)
- [Security Vulnerabilities](#security-vulnerabilities)

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct. We are committed to providing a welcoming and inclusive experience for all contributors.

### Our Standards

- **Be respectful**: Treat all community members with respect and kindness
- **Be constructive**: Provide helpful feedback and suggestions
- **Be collaborative**: Work together to improve the project
- **Be patient**: Help newcomers learn and grow

## Getting Started

### Prerequisites

- **Rust**: 1.70.0 or later
- **Git**: For version control
- **Cargo**: Comes with Rust installation

### Development Setup

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/mocopr.git
   cd mocopr
   ```

3. **Set up the upstream remote**:
   ```bash
   git remote add upstream https://github.com/your-org/mocopr.git
   ```

4. **Install dependencies and build**:
   ```bash
   cargo build --all-features
   ```

5. **Run tests to ensure everything works**:
   ```bash
   cargo test --all-features
   ```

6. **Set up pre-commit hooks** (recommended):
   ```bash
   # Install cargo-husky for git hooks
   cargo install cargo-husky
   
   # Set up hooks
   cargo husky init
   ```

## Contributing Guidelines

### Types of Contributions

We welcome various types of contributions:

- **🐛 Bug fixes**: Fix issues and improve reliability
- **✨ New features**: Add new functionality following MCP specification
- **📚 Documentation**: Improve guides, tutorials, and API docs
- **🧪 Tests**: Add test coverage and improve test quality
- **🔧 Performance**: Optimize performance and reduce resource usage
- **🎨 Examples**: Create helpful examples and tutorials

### Before You Start

1. **Check existing issues**: Look for related issues or discussions
2. **Create an issue**: For new features or major changes, create an issue first
3. **Discuss your approach**: Get feedback before implementing large changes
4. **Check the roadmap**: Ensure your contribution aligns with project goals

## Code Style and Standards

### Rust Style Guidelines

We follow the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/) with some additional conventions:

#### Code Formatting

```bash
# Format code (required before committing)
cargo fmt --all

# Check formatting
cargo fmt --all -- --check
```

#### Linting

```bash
# Run clippy (required before committing)
cargo clippy --all-targets --all-features -- -D warnings

# Fix automatic lint issues
cargo clippy --all-targets --all-features --fix
```

#### Naming Conventions

- **Crates**: `kebab-case` (e.g., `mocopr-core`)
- **Modules**: `snake_case` (e.g., `transport_layer`)
- **Types**: `PascalCase` (e.g., `JsonRpcRequest`)
- **Functions/Variables**: `snake_case` (e.g., `create_request`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `PROTOCOL_VERSION`)
- **Features**: `kebab-case` (e.g., `websocket-transport`)

#### Documentation Standards

- **Public APIs**: Must have rustdoc comments
- **Examples**: Include usage examples in documentation
- **Error conditions**: Document when functions can panic or return errors
- **Safety**: Document unsafe code and safety requirements

```rust
/// Creates a new JSON-RPC request with the specified parameters.
///
/// # Arguments
///
/// * `method` - The method name to call
/// * `params` - Optional parameters for the method
/// * `id` - Optional request ID for tracking
///
/// # Examples
///
/// ```rust
/// use mocopr_core::Protocol;
/// use serde_json::json;
///
/// let request = Protocol::create_request(
///     "tools/call",
///     Some(json!({"name": "calculator", "args": {}})),
///     None,
/// );
/// ```
///
/// # Errors
///
/// This function does not return errors, but serialization might fail.
pub fn create_request(
    method: &str,
    params: Option<Value>,
    id: Option<RequestId>,
) -> JsonRpcRequest {
    // Implementation...
}
```

### Error Handling

- **Use `anyhow::Result<T>`** for most error returns
- **Use `thiserror`** for custom error types
- **Provide meaningful error messages** with context
- **Prefer `?` operator** over `.unwrap()` or `.expect()`
- **Document error conditions** in rustdoc

```rust
use anyhow::{Context, Result};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Invalid message format")]
    InvalidMessage,
}

pub fn connect_to_server(addr: &str) -> Result<Connection> {
    let connection = establish_connection(addr)
        .with_context(|| format!("Failed to connect to server at {}", addr))?;
    
    Ok(connection)
}
```

### Async Programming

- **Use `async/await`** throughout the codebase
- **Use `tokio`** for async runtime
- **Use `async-trait`** for trait definitions with async methods
- **Handle cancellation** properly in long-running operations

```rust
use async_trait::async_trait;
use tokio::time::{timeout, Duration};

#[async_trait]
pub trait MessageHandler {
    async fn handle_message(&self, message: JsonRpcMessage) -> Result<JsonRpcResponse>;
}

pub async fn send_with_timeout(
    transport: &impl Transport,
    message: JsonRpcMessage,
) -> Result<JsonRpcResponse> {
    timeout(Duration::from_secs(30), transport.send(message))
        .await
        .context("Request timed out")?
}
```

## Testing

### Test Categories

1. **Unit Tests**: Test individual functions and modules
2. **Integration Tests**: Test component interactions
3. **End-to-End Tests**: Test complete workflows
4. **Performance Tests**: Benchmark critical paths
5. **Security Tests**: Test security boundaries

### Running Tests

```bash
# Run all tests
cargo test --all-features

# Run specific test suite
cargo test --test integration_tests

# Run with output
cargo test -- --nocapture

# Run performance benchmarks
cargo bench

# Run security audit
cargo audit
```

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_request_creation() {
        let request = Protocol::create_request(
            "test/method",
            Some(json!({"param": "value"})),
            Some(RequestId::from("test-id")),
        );
        
        assert_eq!(request.method, "test/method");
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, Some(RequestId::from("test-id")));
    }
    
    #[tokio::test]
    async fn test_error_handling() {
        let result = some_function_that_fails().await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected error"));
    }
}
```

### Test Coverage

- **Aim for 90%+ code coverage**
- **Include edge cases and error conditions**
- **Test concurrent scenarios**
- **Use property-based testing where appropriate**

```bash
# Generate coverage report
cargo llvm-cov --all-features --workspace --html

# View coverage
open target/llvm-cov/html/index.html
```

## Documentation

### Types of Documentation

1. **API Documentation**: rustdoc comments for public APIs
2. **Guides**: High-level guides in `docs/guides/`
3. **Tutorials**: Step-by-step tutorials in `docs/tutorials/`
4. **Examples**: Working code examples in `examples/`
5. **README**: Project overview and quick start

### Building Documentation

```bash
# Build API documentation
cargo doc --all-features --no-deps

# Build and open documentation
cargo doc --all-features --no-deps --open

# Check documentation
cargo doc --all-features --no-deps --document-private-items
```

### Documentation Guidelines

- **Write for your audience**: Consider who will read the documentation
- **Include examples**: Show how to use the APIs
- **Keep it up-to-date**: Update docs when code changes
- **Test examples**: Ensure code examples actually work

## Pull Request Process

### Before Submitting

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following the guidelines above

3. **Run the full test suite**:
   ```bash
   cargo test --all-features
   cargo clippy --all-targets --all-features -- -D warnings
   cargo fmt --all -- --check
   ```

4. **Update documentation** if needed

5. **Add tests** for new functionality

6. **Update CHANGELOG.md** if applicable

### Submitting the PR

1. **Push your branch**:
   ```bash
   git push origin feature/your-feature-name
   ```

2. **Create a Pull Request** on GitHub

3. **Fill out the PR template** completely

4. **Request review** from maintainers

### PR Requirements

- ✅ **All tests pass**
- ✅ **Code is formatted** (`cargo fmt`)
- ✅ **No lint warnings** (`cargo clippy`)
- ✅ **Documentation updated** if needed
- ✅ **Tests added** for new functionality
- ✅ **CHANGELOG updated** for user-facing changes

### Review Process

1. **Automated checks** run on all PRs
2. **Maintainer review** for code quality and design
3. **Address feedback** if requested
4. **Approval and merge** by maintainers

## Issue Reporting

### Bug Reports

When reporting bugs, please include:

- **Environment**: Rust version, OS, MoCoPr version
- **Steps to reproduce**: Clear reproduction steps
- **Expected behavior**: What should happen
- **Actual behavior**: What actually happens
- **Code samples**: Minimal reproduction case
- **Logs/errors**: Relevant error messages

### Feature Requests

When requesting features, please include:

- **Use case**: Why is this feature needed?
- **Description**: What should the feature do?
- **Alternatives**: What alternatives have you considered?
- **Implementation**: Any implementation ideas?

### Using Issue Templates

We provide issue templates for:

- 🐛 **Bug Report**
- ✨ **Feature Request**
- 📚 **Documentation Issue**
- ❓ **Question/Support**

## Security Vulnerabilities

### Reporting Security Issues

**DO NOT** open public issues for security vulnerabilities. Instead:

1. **Email**: Send details to [security@mocopr.dev](mailto:security@mocopr.dev)
2. **Include**: Detailed description and reproduction steps
3. **Response**: We'll respond within 48 hours
4. **Disclosure**: We'll coordinate responsible disclosure

### Security Best Practices

When contributing, keep security in mind:

- **Validate all inputs** from external sources
- **Sanitize file paths** and prevent directory traversal
- **Use secure defaults** for configuration
- **Avoid eval-like operations** without sandboxing
- **Be cautious with unsafe code**

## Community

### Getting Help

- **GitHub Discussions**: For questions and community discussion
- **Discord Server**: Real-time chat with maintainers and community
- **Stack Overflow**: Tag questions with `mocopr`

### Staying Updated

- **Watch the repository** for notifications
- **Follow releases** for updates
- **Join the mailing list** for announcements

## Recognition

Contributors are recognized in:

- **CONTRIBUTORS.md**: All contributors listed
- **Release notes**: Major contributions highlighted
- **Hall of Fame**: Outstanding contributors featured

Thank you for contributing to MoCoPr! 🚀
