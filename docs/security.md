# Security Guidelines and Best Practices

This document outlines security considerations, best practices, and guidelines for using and contributing to MoCoPr.

## 🔒 Security Overview

MoCoPr takes security seriously and implements multiple layers of protection:

- **Input Validation**: All inputs are validated against schemas
- **Transport Security**: Support for TLS encryption
- **Access Control**: Configurable authentication and authorization
- **Resource Isolation**: Safe resource access patterns
- **Error Handling**: Secure error messages without information leakage
- **Audit Logging**: Comprehensive security event logging

## 🛡️ Security Architecture

### Transport Layer Security

#### stdio Transport
- **Process Isolation**: Each MCP server runs in its own process
- **Privilege Separation**: Servers should run with minimal privileges
- **Input/Output Validation**: All stdio communication is validated

```rust
use mocopr::security::*;

let server = McpServer::builder()
    .with_security_policy(SecurityPolicy::strict())
    .with_input_validation(true)
    .with_privilege_dropping("mcp-user")
    .build()?;
```

#### WebSocket Transport
- **TLS Encryption**: Mandatory for production deployments
- **Origin Validation**: Restrict allowed origins
- **Rate Limiting**: Prevent abuse and DoS attacks
- **Authentication**: Token-based or certificate authentication

```rust
let websocket_config = WebSocketConfig::builder()
    .with_tls(TlsConfig::from_pem_files("cert.pem", "key.pem"))
    .with_origin_validation(&["https://trusted-domain.com"])
    .with_rate_limiting(RateLimit::per_minute(100))
    .with_auth(AuthMethod::Bearer)
    .build();

server.run_websocket_secure(websocket_config).await?;
```

#### HTTP Transport
- **HTTPS Only**: No plain HTTP in production
- **CORS Configuration**: Properly configured cross-origin policies
- **Request Size Limits**: Prevent memory exhaustion attacks
- **Authentication Headers**: Secure token validation

```rust
let http_config = HttpConfig::builder()
    .with_tls_required(true)
    .with_cors(CorsConfig::restrictive())
    .with_max_request_size(1024 * 1024) // 1MB limit
    .with_timeout(Duration::from_secs(30))
    .build();
```

### Authentication and Authorization

#### Token-Based Authentication

```rust
use mocopr::auth::*;

let auth_middleware = AuthMiddleware::builder()
    .with_token_validation(TokenValidator::jwt(&secret_key))
    .with_token_expiry(Duration::from_hours(24))
    .with_refresh_tokens(true)
    .build();

let server = McpServer::builder()
    .with_middleware(auth_middleware)
    .build()?;
```

#### Role-Based Access Control (RBAC)

```rust
let rbac = RoleBasedAccessControl::builder()
    .with_role("admin", &["*"])
    .with_role("user", &["tools:read", "resources:read"])
    .with_role("service", &["tools:call", "resources:read:public/*"])
    .build();

let server = McpServer::builder()
    .with_authorization(rbac)
    .build()?;
```

### Input Validation and Sanitization

#### JSON Schema Validation

```rust
use mocopr::validation::*;

let validator = JsonSchemaValidator::builder()
    .with_strict_mode(true)
    .with_max_depth(10)
    .with_max_properties(100)
    .with_string_length_limit(10_000)
    .build();

let server = McpServer::builder()
    .with_input_validator(validator)
    .build()?;
```

#### Resource URI Validation

```rust
let uri_validator = UriValidator::builder()
    .with_allowed_schemes(&["file", "memory", "https"])
    .with_path_traversal_protection(true)
    .with_length_limit(2048)
    .build();

let resource_handler = ResourceHandler::builder()
    .with_uri_validator(uri_validator)
    .with_access_control(ResourceAccessControl::sandboxed("/safe/directory"))
    .build();
```

### Resource Security

#### File System Access

```rust
use mocopr::resources::security::*;

let file_handler = FileResourceHandler::builder()
    .with_base_directory("/safe/data/directory")
    .with_read_only(true)
    .with_symlink_following(false)
    .with_file_size_limit(100 * 1024 * 1024) // 100MB
    .with_allowed_extensions(&["txt", "json", "md"])
    .build();
```

#### Memory Resource Protection

```rust
let memory_handler = MemoryResourceHandler::builder()
    .with_memory_limit(50 * 1024 * 1024) // 50MB limit
    .with_key_validation(KeyValidator::alphanumeric_only())
    .with_ttl(Duration::from_hours(1))
    .build();
```

### Error Handling Security

#### Secure Error Messages

```rust
use mocopr::error::SecurityLevel;

let error_handler = ErrorHandler::builder()
    .with_security_level(SecurityLevel::Production)
    .with_error_sanitization(true)
    .with_stack_trace_filtering(true)
    .build();

// In production, this will not leak internal details
fn handle_request(request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
    match process_request(request) {
        Ok(response) => Ok(response),
        Err(e) => {
            // Log full error internally
            tracing::error!("Request processing failed: {:?}", e);
            
            // Return sanitized error to client
            Err(Error::internal_error("Request processing failed"))
        }
    }
}
```

## 🔍 Security Audit Checklist

### Deployment Security

- [ ] **TLS Configuration**: All network communication uses TLS 1.2+
- [ ] **Certificate Validation**: Valid certificates from trusted CA
- [ ] **Key Management**: Private keys stored securely
- [ ] **Firewall Rules**: Restrict access to necessary ports only
- [ ] **Process Isolation**: Servers run in containers or separate processes
- [ ] **Privilege Dropping**: Servers run with minimal required privileges
- [ ] **Resource Limits**: Memory, CPU, and file descriptor limits set
- [ ] **Monitoring**: Security events are logged and monitored

### Code Security

- [ ] **Input Validation**: All external inputs validated
- [ ] **Output Sanitization**: No sensitive data in error messages
- [ ] **Dependency Audit**: Regular security audits of dependencies
- [ ] **Secret Management**: No hardcoded secrets in code
- [ ] **Unsafe Code Review**: All unsafe code blocks reviewed
- [ ] **Error Handling**: Proper error handling without information leakage

### Configuration Security

- [ ] **Default Security**: Secure defaults for all configurations
- [ ] **Configuration Validation**: Startup validation of security settings
- [ ] **Secret Storage**: Secrets stored in secure configuration management
- [ ] **Environment Separation**: Different configs for dev/staging/prod
- [ ] **Access Controls**: Restricted access to configuration files

## 🚨 Common Security Pitfalls

### Avoid These Patterns

#### ❌ Path Traversal Vulnerabilities

```rust
// BAD: Allows path traversal
fn read_file(path: &str) -> Result<String> {
    std::fs::read_to_string(path) // Dangerous!
}

// GOOD: Validate and sanitize paths
fn read_file_safe(path: &str) -> Result<String> {
    let sanitized = sanitize_path(path)?;
    let full_path = base_directory().join(sanitized);
    
    if !full_path.starts_with(base_directory()) {
        return Err(Error::access_denied("Path traversal attempted"));
    }
    
    std::fs::read_to_string(full_path)
}
```

#### ❌ Information Disclosure

```rust
// BAD: Leaks internal information
fn handle_error(e: InternalError) -> JsonRpcError {
    JsonRpcError {
        code: -32603,
        message: format!("Database connection failed: {}", e), // Leaks details!
        data: None,
    }
}

// GOOD: Generic error messages
fn handle_error_safe(e: InternalError) -> JsonRpcError {
    tracing::error!("Internal error: {:?}", e); // Log internally
    
    JsonRpcError {
        code: -32603,
        message: "Internal server error".to_string(), // Generic message
        data: None,
    }
}
```

#### ❌ Unvalidated Deserialization

```rust
// BAD: Unrestricted deserialization
fn handle_request(data: &[u8]) -> Result<Request> {
    serde_json::from_slice(data) // No validation!
}

// GOOD: Validated deserialization
fn handle_request_safe(data: &[u8]) -> Result<Request> {
    if data.len() > MAX_REQUEST_SIZE {
        return Err(Error::request_too_large());
    }
    
    let value: serde_json::Value = serde_json::from_slice(data)?;
    validate_json_schema(&value, &REQUEST_SCHEMA)?;
    
    serde_json::from_value(value)
}
```

## 🔧 Security Configuration

### Production Security Template

```rust
use mocopr::prelude::*;
use mocopr::security::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize security logging
    let security_logger = SecurityLogger::builder()
        .with_audit_trail(true)
        .with_sensitive_data_filtering(true)
        .build();

    // Configure security policies
    let security_policy = SecurityPolicy::builder()
        .with_input_validation(InputValidationPolicy::strict())
        .with_output_sanitization(true)
        .with_rate_limiting(RateLimitPolicy::production())
        .with_authentication_required(true)
        .with_authorization_enabled(true)
        .build();

    // Build secure server
    let server = McpServer::builder()
        .with_security_policy(security_policy)
        .with_logger(security_logger)
        .with_middleware(AuthMiddleware::jwt(&load_secret()?))
        .with_middleware(RateLimitMiddleware::per_client(100))
        .with_middleware(CorsMiddleware::restrictive())
        .with_middleware(SecurityHeadersMiddleware::default())
        .build()?;

    // Start with secure transport
    let tls_config = TlsConfig::builder()
        .with_cert_chain_file("cert.pem")
        .with_private_key_file("key.pem")
        .with_min_tls_version(TlsVersion::V1_2)
        .with_cipher_suites(&SECURE_CIPHER_SUITES)
        .build()?;

    server.run_https("0.0.0.0:443", tls_config).await?;
    Ok(())
}

fn load_secret() -> Result<Vec<u8>> {
    // Load from secure storage, environment, or config management
    std::env::var("JWT_SECRET")
        .context("JWT_SECRET environment variable not set")?
        .into_bytes()
        .pipe(Ok)
}
```

### Development Security

```rust
// Development configuration with relaxed security for testing
let dev_config = SecurityPolicy::builder()
    .with_input_validation(InputValidationPolicy::permissive())
    .with_detailed_errors(true) // OK for development
    .with_cors_allow_all(true) // OK for development
    .with_authentication_required(false) // OK for development
    .build();
```

## 📊 Security Monitoring

### Audit Logging

```rust
use mocopr::audit::*;

let audit_logger = AuditLogger::builder()
    .with_events(&[
        AuditEvent::Authentication,
        AuditEvent::Authorization,
        AuditEvent::ResourceAccess,
        AuditEvent::ToolExecution,
        AuditEvent::SecurityViolation,
    ])
    .with_structured_logging(true)
    .with_retention_policy(RetentionPolicy::days(90))
    .build();
```

### Metrics and Alerting

```rust
use mocopr::metrics::security::*;

let security_metrics = SecurityMetrics::builder()
    .with_authentication_failures(true)
    .with_rate_limit_violations(true)
    .with_request_size_monitoring(true)
    .with_error_rate_monitoring(true)
    .build();

// Set up alerts
let alerting = AlertingConfig::builder()
    .with_threshold("auth_failures_per_minute", 10)
    .with_threshold("rate_limit_violations_per_minute", 50)
    .with_threshold("error_rate_percentage", 5.0)
    .build();
```

## 🔐 Cryptographic Security

### Key Management

```rust
use mocopr::crypto::*;

// Use secure key derivation
let key = KeyDerivation::pbkdf2()
    .with_salt(&random_salt())
    .with_iterations(100_000)
    .derive_key(&password)?;

// Or use hardware security modules
let hsm_key = HsmKeyManager::new(hsm_config)
    .load_key("mcp-server-key")?;
```

### Token Security

```rust
// Secure JWT configuration
let jwt_config = JwtConfig::builder()
    .with_algorithm(JwtAlgorithm::RS256) // Asymmetric keys preferred
    .with_issuer("mcp-server")
    .with_audience("mcp-clients")
    .with_expiry(Duration::from_hours(1)) // Short-lived tokens
    .with_refresh_tokens(true)
    .with_key_rotation(Duration::from_days(30))
    .build();
```

## 📋 Security Testing

### Automated Security Testing

```bash
# Run security audit
cargo audit

# Check for unsafe code
cargo geiger

# Run security-focused tests
cargo test security::

# Fuzz testing
cargo fuzz run protocol_parser

# Static analysis
cargo clippy -- -W clippy::all -W clippy::security
```

### Penetration Testing Checklist

- [ ] **Input Fuzzing**: Fuzz all input parsers
- [ ] **Authentication Bypass**: Test auth mechanisms
- [ ] **Authorization Escalation**: Test permission boundaries  
- [ ] **Resource Exhaustion**: Test DoS resistance
- [ ] **Injection Attacks**: Test for various injection types
- [ ] **Path Traversal**: Test file access controls
- [ ] **TLS Configuration**: Test encryption and certificate handling

## 🚨 Incident Response

### Security Incident Handling

1. **Detection**: Monitor for security events
2. **Assessment**: Evaluate impact and scope
3. **Containment**: Isolate affected systems
4. **Investigation**: Analyze attack vectors
5. **Recovery**: Restore secure operations
6. **Lessons Learned**: Improve security measures

### Emergency Contacts

- **Security Team**: security@mocopr.dev
- **On-Call**: +1-XXX-XXX-XXXX
- **Incident Commander**: incidents@mocopr.dev

## 📚 Security Resources

### Internal Resources

- [Security Architecture Design](docs/security/architecture.md)
- [Threat Model](docs/security/threat-model.md)
- [Security Testing Guide](docs/security/testing.md)
- [Incident Response Playbook](docs/security/incident-response.md)

### External Resources

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [JSON-RPC Security Considerations](https://www.jsonrpc.org/security)
- [WebSocket Security](https://tools.ietf.org/html/rfc6455#section-10)

---

**Remember**: Security is everyone's responsibility. When in doubt, choose the more secure option and ask for security review.
