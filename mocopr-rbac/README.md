# MoCoPr RBAC

Role-Based Access Control (RBAC) integration for MoCoPr MCP servers, built on the powerful [role-system](https://crates.io/crates/role-system) crate.

## Features

- **🔒 Fine-grained Permissions**: Control access to specific tools, resources, and prompts
- **🏗️ Hierarchical Roles**: Support for role inheritance (admin > power_user > user)
- **⚙️ Conditional Permissions**: Context-based access control (time, location, trust level)
- **👥 Multiple Subject Types**: Users, services, devices, and groups
- **🚀 Async Support**: Full async/await compatibility with MoCoPr
- **📋 Audit Logging**: Comprehensive security event logging
- **💾 Persistence**: Optional role/permission persistence
- **🔧 Easy Integration**: Drop-in middleware for existing MCP servers

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
mocopr-rbac = "0.1"
mocopr-server = "0.1"
```

### Basic Usage

```rust
use mocopr_rbac::prelude::*;
use mocopr_server::McpServer;

#[tokio::main]
async fn main() -> Result<()> {
    // Create RBAC middleware with default roles
    let rbac = RbacMiddleware::builder()
        .with_default_roles()
        .with_audit_logging(true)
        .build()
        .await?;

    // Build MCP server with RBAC
    let server = McpServer::builder()
        .name("Secure MCP Server")
        .with_middleware(rbac)
        .build()?;

    server.run_stdio().await
}
```

### Custom Roles and Permissions

```rust
let rbac = RbacMiddleware::builder()
    .with_role("data_analyst", &[
        "list:tools",
        "call:tools:analytics/*",
        "read:resources:data/*"
    ])
    .with_role("admin", &[
        "call:tools:*",
        "read:resources:*",
        "manage:server"
    ])
    .build()
    .await?;
```

### Conditional Permissions

```rust
use mocopr_rbac::context::ContextConditions;

let rbac = RbacMiddleware::builder()
    .with_default_roles()
    
    // Admin tools only during business hours
    .with_conditional_permission(
        "power_user",
        "call:tools:admin/*",
        ContextConditions::business_hours_only()
    )
    
    // Sensitive operations for high-trust clients only
    .with_conditional_permission(
        "admin",
        "call:tools:dangerous/*",
        |context| {
            context.get("trust_level") == Some(&"high".to_string())
        }
    )
    .build()
    .await?;
```

## Permission Model

Permissions follow the format `action:resource_type:resource_id`:

### Common Patterns

- `list:tools` - List available tools
- `call:tools:calculator` - Execute the calculator tool
- `call:tools:*` - Execute any tool
- `read:resources:file/*` - Read any file resource
- `read:resources:data/sensitive.txt` - Read specific file
- `manage:server` - Server administration

### Default Roles

When using `.with_default_roles()`, these roles are created:

#### Guest
- `list:tools`
- `list:resources`

#### User (inherits from Guest)
- `call:tools`
- `read:resources`

#### Power User (inherits from User)  
- `*:tools`
- `*:resources`
- `list:prompts`
- `get:prompts`

#### Admin (inherits from Power User)
- `*:*` (super admin - access to everything)

## Subject Types

The system supports different types of subjects:

```rust
// Human users
let user = MocoPrSubject::user("alice");

// Automated services
let service = MocoPrSubject::service("data-processor");

// IoT devices
let device = MocoPrSubject::device("sensor-001");

// Groups
let group = MocoPrSubject::group("engineering-team");

// Custom types
let custom = MocoPrSubject::custom("ai-agent", "llm");
```

## Context-Based Permissions

Context extractors provide runtime information for conditional permissions:

### Built-in Context

- `timestamp` - Current timestamp (RFC3339)
- `business_hours` - "true" if 9 AM - 5 PM
- `day_of_week` - Day name (Monday, Tuesday, etc.)
- `is_weekend` - "true" on weekends
- `method` - MCP method being called
- `user_id` - Subject identifier
- `client_ip` - Client IP address (if provided)

### Custom Context

```rust
let rbac = RbacMiddleware::builder()
    .with_context_extractor(
        ExtendedContextExtractor::new()
            .with_trust_level_extractor()
            .with_location_extractor()
    )
    .build()
    .await?;
```

## Client Authentication

To authenticate with an RBAC-enabled server, include auth parameters in requests:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "calculator",
    "arguments": {"a": 5, "b": 3},
    "auth": {
      "subject_id": "alice",
      "subject_type": "user"
    },
    "context": {
      "trust_level": "high",
      "location": "office"
    }
  }
}
```

## Configuration

### File-Based Configuration

```rust
use mocopr_rbac::config::RbacConfig;

// Load from file
let config = RbacConfig::from_file("rbac.json")?;

// Or create programmatically
let config = RbacConfig::production_template();
config.to_file("rbac.json")?;
```

### Environment-Specific Configs

```rust
// Development
let config = RbacConfig::development();

// Production
let config = RbacConfig::production_template();
```

## Advanced Features

### Role Hierarchy

```rust
// Set up inheritance chain: admin > manager > user > guest
rbac.add_role_inheritance("admin", "manager").await?;
rbac.add_role_inheritance("manager", "user").await?;
rbac.add_role_inheritance("user", "guest").await?;
```

### Temporary Role Elevation

```rust
// Elevate user to admin for 1 hour
rbac.elevate_role(
    &user,
    "admin",
    Some(Duration::from_hours(1))
).await?;
```

### Audit Logging

When audit logging is enabled, all permission checks are logged:

```
INFO rbac: Permission check subject=alice action=call resource=calculator result=granted
WARN rbac: Permission check subject=bob action=call resource=admin/restart result=denied
```

## Examples

See the `examples/rbac-example` directory for a complete working example demonstrating:

- Custom role definitions
- Conditional permissions  
- Context-based access control
- Different security levels for tools
- Audit logging

## Security Considerations

- **Fail-Safe Defaults**: All permissions are denied by default
- **Explicit Grants**: Permissions must be explicitly granted
- **Context Validation**: Conditional permissions validate runtime context
- **Audit Trail**: All access attempts are logged when audit is enabled
- **Input Validation**: All inputs are validated and sanitized

## Performance

- **Built-in Caching**: Permission results are cached to improve performance
- **Thread-Safe**: Uses lock-free data structures for concurrent access
- **Efficient Hierarchy**: Smart role hierarchy traversal with cycle detection
- **Lazy Evaluation**: Permissions calculated only when needed

## Integration with Other Frameworks

The RBAC middleware can be combined with other MoCoPr middleware:

```rust
let server = McpServer::builder()
    .with_middleware(AuthMiddleware::jwt(&secret))  // Authentication
    .with_middleware(rbac)                          // Authorization
    .with_middleware(RateLimitMiddleware::new())    // Rate limiting
    .with_middleware(LoggingMiddleware::new())      // Logging
    .build()?;
```

## Contributing

Contributions are welcome! Please ensure that:

1. All tests pass: `cargo test`
2. Code is formatted: `cargo fmt`
3. No clippy warnings: `cargo clippy`
4. Documentation is updated for new features

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
