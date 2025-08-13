# Production Deployment Guide

This guide covers deploying MoCoPr-based MCP servers in production environments with comprehensive security, monitoring, and scaling capabilities. MoCoPr provides enterprise-ready features including middleware, multiple transport protocols, built-in monitoring, and security systems.

## Production Readiness Checklist

### Essential Components

- [ ] **Security**: Authentication, authorization, input validation
- [ ] **Monitoring**: Metrics, logging, health checks, alerting
- [ ] **Performance**: Resource limits, connection pooling, caching
- [ ] **Reliability**: Error handling, circuit breakers, graceful shutdown
- [ ] **Operations**: Configuration management, deployment automation

## Deployment Architecture

### Recommended Production Stack

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Load Balancer │────│   MCP Servers   │────│    Database     │
│   (nginx/envoy) │    │   (multiple)    │    │  (PostgreSQL)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │              ┌─────────────────┐              │
         └──────────────│   Monitoring    │──────────────┘
                        │ (Prometheus +   │
                        │  Grafana +      │
                        │  Alertmanager)  │
                        └─────────────────┘
```

## Security Configuration

### 1. Production Security Setup

```rust
use mocopr_server::prelude::*;
use mocopr_server::middleware::*;

#[derive(serde::Deserialize)]
struct SecurityConfig {
    api_keys: Vec<String>,
    max_requests_per_minute: u32,
    max_request_size: usize,
}

async fn build_secure_server(config: SecurityConfig) -> anyhow::Result<McpServer> {
    let server = McpServerBuilder::new()
        .with_info("production-server", "1.0.0")
        .with_tools()
        .with_resources()
        .with_prompts()

        // Add middleware directly to the builder
        .with_middleware(LoggingMiddleware::new()
            .with_requests(true)
            .with_responses(false)
            .with_timing(true))

        .with_middleware(RateLimitMiddleware::new(
            config.max_requests_per_minute,
            std::time::Duration::from_secs(60)
        ))

        .with_middleware(AuthMiddleware::new()
            .with_api_keys(config.api_keys.clone())
            .with_required(true))

        // Enable monitoring system
        .with_monitoring()

        // Configure transport options
        .with_bind_address("0.0.0.0", 8080)
        .with_http_transport()
        .with_websocket_transport()

        // Add your production tools and resources here
        .build()?;

    Ok(server)
}

// Middleware is implemented and ready to use:
fn create_production_middleware() -> Vec<Box<dyn Middleware>> {
    vec![
        // Request logging with timing
        Box::new(LoggingMiddleware::new()
            .with_requests(true)
            .with_responses(false)
            .with_timing(true)),

        // Rate limiting
        Box::new(RateLimitMiddleware::new(
            1000, // 1000 requests
            std::time::Duration::from_secs(60), // per minute
        )),

        // Authentication
        Box::new(AuthMiddleware::new()
            .with_api_keys(vec![
                "your-api-key-1".to_string(),
                "your-api-key-2".to_string(),
            ])),

        // Performance metrics
        Box::new(MetricsMiddleware::new()),
    ]
}
```

```

### 2. Transport Configuration

MoCoPr now supports multiple transport protocols out of the box:

```rust
// Configure multiple transports
let server = McpServerBuilder::new()
    .with_info("multi-transport-server", "1.0.0")
    .with_bind_address("0.0.0.0", 8080)

    // Enable HTTP REST API
    .with_http_transport()

    // Enable WebSocket for real-time communication
    .with_websocket_transport()

    .build()?;

// Server will run on:
// - HTTP: http://localhost:8080/mcp (POST requests)
// - WebSocket: ws://localhost:8080/mcp/ws
// - Stdio: For process-based communication

// Start with all configured transports
server.run().await?;

// Or run specific transports:
// server.run_stdio().await?;           // Just stdio
// server.run_http("0.0.0.0:8080").await?;        // Just HTTP
// server.run_websocket("0.0.0.0:8080").await?;   // Just WebSocket
```

### 3. TLS Configuration

For production deployments, use a reverse proxy (nginx, Envoy) for TLS termination:

```nginx
server {
    listen 443 ssl http2;
    server_name your-mcp-server.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    # Proxy HTTP requests
    location /mcp {
        proxy_pass http://localhost:8080/mcp;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Proxy WebSocket connections
    location /mcp/ws {
        proxy_pass http://localhost:8080/mcp/ws;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

```

### 3. Environment-based Configuration

Create `config/production.toml`:

```toml
[server]
name = "production-mcp-server"
version = "1.0.0"
bind_address = "0.0.0.0"
port = 8443

[security]
jwt_secret_env = "JWT_SECRET"
tls_cert_path = "/etc/ssl/certs/server.crt"
tls_key_path = "/etc/ssl/private/server.key"
max_request_size = 1048576  # 1MB
rate_limit_per_minute = 1000
allowed_origins = ["https://yourdomain.com"]

[database]
url_env = "DATABASE_URL"
max_connections = 20
min_connections = 5
connection_timeout_seconds = 30

[monitoring]
metrics_port = 9090
health_check_port = 8080
log_level = "info"
enable_tracing = true

[performance]
worker_threads = 0  # Use all available cores
max_blocking_threads = 512
request_timeout_seconds = 30
```

Load configuration:

```rust
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AppConfig {
    server: ServerConfig,
    security: SecurityConfig,
    database: DatabaseConfig,
    monitoring: MonitoringConfig,
    performance: PerformanceConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}",
                std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".into())
            )))
            .add_source(Environment::with_prefix("APP"))
            .build()?;

        config.try_deserialize()
    }
}
```

## Monitoring and Observability

### 1. Metrics Collection

> **Note**: Built-in metrics are not yet implemented. Use external monitoring tools like Prometheus with custom metrics for now.

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

// Custom metrics tracking (until built-in metrics are available)
#[derive(Clone)]
pub struct SimpleMetrics {
    requests_total: Arc<AtomicU64>,
    errors_total: Arc<AtomicU64>,
}

impl SimpleMetrics {
    pub fn new() -> Self {
        Self {
            requests_total: Arc::new(AtomicU64::new(0)),
            errors_total: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn increment_requests(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_errors(&self) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_requests(&self) -> u64 {
        self.requests_total.load(Ordering::Relaxed)
    }

    pub fn get_errors(&self) -> u64 {
        self.errors_total.load(Ordering::Relaxed)
    }
}

// Implement metrics in your tool handlers
impl ToolHandler for YourTool {
    async fn call(&self, arguments: Option<serde_json::Value>) -> Result<ToolsCallResponse> {
        self.metrics.increment_requests();
        let start = std::time::Instant::now();

        let result = self.execute_tool_logic(arguments).await;

        if result.is_err() {
            self.metrics.increment_errors();
        }

        let duration = start.elapsed();
        tracing::info!("Tool call completed in {:?}", duration);

        result
    }
}
```

### 2. Health Checks

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy { checks: Vec<HealthCheck> },
    Degraded { checks: Vec<HealthCheck> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: String,
    pub message: String,
}

impl HealthCheck {
    pub fn healthy(name: &str, message: &str) -> Self {
        Self {
            name: name.to_string(),
            status: "healthy".to_string(),
            message: message.to_string(),
        }
    }

    pub fn unhealthy(name: &str, message: &str) -> Self {
        Self {
            name: name.to_string(),
            status: "unhealthy".to_string(),
            message: message.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct HealthChecker {
    // Add your dependencies here (database pools, etc.)
}

impl HealthChecker {
    pub async fn check_health(&self) -> HealthStatus {
        let mut checks = Vec::new();

        // Check memory usage
        checks.push(self.check_memory_usage().await);

        // Check disk space
        checks.push(self.check_disk_space().await);

        // Add more checks as needed
        if checks.iter().all(|c| c.status == "healthy") {
            HealthStatus::Healthy { checks }
        } else {
            HealthStatus::Degraded { checks }
        }
    }

    async fn check_memory_usage(&self) -> HealthCheck {
        // Implement basic memory check
        HealthCheck::healthy("memory", "Memory usage normal")
    }

    async fn check_disk_space(&self) -> HealthCheck {
        // Implement basic disk space check
        HealthCheck::healthy("disk", "Disk space sufficient")
    }
}

// Create a simple health check tool
pub struct HealthTool {
    checker: HealthChecker,
}

#[async_trait]
impl ToolHandler for HealthTool {
    async fn tool(&self) -> Tool {
        Tool::new("health_check", json!({"type": "object"}))
            .with_description("Check server health status")
    }

    async fn call(&self, _arguments: Option<serde_json::Value>) -> Result<ToolsCallResponse> {
        let status = self.checker.check_health().await;
        let status_json = serde_json::to_string(&status)?;

        Ok(ToolsCallResponse::success(vec![
            Content::Text(TextContent::new(status_json))
        ]))
    }
}
```

```

### 3. Structured Logging

```rust
use tracing::{info, error, warn, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn setup_logging(config: &MonitoringConfig) -> anyhow::Result<()> {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .json(); // Use JSON format for structured logging

    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(config.log_level.parse()?)
        .from_env()?;

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(filter)
        .init();

    info!("Logging initialized with level: {}", config.log_level);
    Ok(())
}

#[derive(Clone)]
pub struct StructuredLoggingMiddleware;

#[async_trait::async_trait]
impl Middleware for StructuredLoggingMiddleware {
    async fn handle(&self, request: JsonRpcRequest, next: Next) -> Result<JsonRpcResponse> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let start = std::time::Instant::now();

        // Create a span for this request
        let span = tracing::info_span!(
            "mcp_request",
            request.id = %request_id,
            method = %request.method,
            jsonrpc = %request.jsonrpc
        );

        async move {
            info!("Processing request");

            let result = next.run(request).await;
            let duration = start.elapsed();

            match &result {
                Ok(response) => {
                    info!(
                        duration_ms = duration.as_millis(),
                        "Request completed successfully"
                    );
                }
                Err(error) => {
                    error!(
                        duration_ms = duration.as_millis(),
                        error = %error,
                        "Request failed"
                    );
                }
            }

            result
        }.instrument(span).await
    }
}
```

## Docker Deployment

### 1. Multi-stage Dockerfile

```dockerfile
# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY mocopr-* ./mocopr-*/
COPY src ./src

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create application user
RUN groupadd -r mcpuser && useradd -r -g mcpuser mcpuser

# Copy the binary
COPY --from=builder /app/target/release/production-server /usr/local/bin/mcp-server

# Copy configuration (if any)
COPY config/ /app/config/

# Set permissions
RUN chown -R mcpuser:mcpuser /app

# Switch to non-root user
USER mcpuser

# Health check (basic process check since HTTP is not yet supported)
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD pgrep mcp-server || exit 1

# Run the server
WORKDIR /app
CMD ["mcp-server"]
```

### 2. Entrypoint Script

```bash
#!/bin/bash
set -e

# Wait for dependencies (database, etc.)
echo "Waiting for dependencies..."
while ! nc -z database 5432; do
    echo "Waiting for database..."
    sleep 1
done

echo "Dependencies ready. Starting MCP server..."

# Set environment variables for configuration
export ENVIRONMENT=${ENVIRONMENT:-production}
export RUST_LOG=${RUST_LOG:-info}

# Run the server
exec "$@"
```

### 3. Docker Compose for Production

```yaml
version: '3.8'

services:
  mcp-server:
    build:
      context: .
      dockerfile: Dockerfile
    image: mcp-server:latest
    environment:
      - RUST_LOG=info
      - ENVIRONMENT=production
    volumes:
      - ./data:/app/data
      - ./config:/app/config:ro
    restart: unless-stopped
    deploy:
      resources:
        limits:
          cpus: '1.0'
          memory: 512M
        reservations:
          cpus: '0.25'
          memory: 256M

  # Optional: Add nginx for future HTTP support
  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/ssl:ro
    depends_on:
      - mcp-server
    restart: unless-stopped

  # Optional: Basic monitoring
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml:ro
    restart: unless-stopped

volumes:
  app_data:
```

## Kubernetes Deployment

### 1. Kubernetes Manifests

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mcp-server
  labels:
    app: mcp-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: mcp-server
  template:
    metadata:
      labels:
        app: mcp-server
    spec:
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
      containers:
      - name: mcp-server
        image: mcp-server:latest
        ports:
        - containerPort: 8443
          name: https
        - containerPort: 8080
          name: health
        - containerPort: 9090
          name: metrics
        env:
        - name: ENVIRONMENT
          value: "production"
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: mcp-secrets
              key: database-url
        - name: JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: mcp-secrets
              key: jwt-secret
        resources:
          requests:
            cpu: 500m
            memory: 512Mi
          limits:
            cpu: 2
            memory: 1Gi
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
        volumeMounts:
        - name: config
          mountPath: /app/config
          readOnly: true
        - name: tls
          mountPath: /etc/ssl
          readOnly: true
      volumes:
      - name: config
        configMap:
          name: mcp-config
      - name: tls
        secret:
          secretName: mcp-tls

---
apiVersion: v1
kind: Service
metadata:
  name: mcp-server-service
spec:
  selector:
    app: mcp-server
  ports:
  - name: https
    port: 443
    targetPort: 8443
  - name: health
    port: 8080
    targetPort: 8080
  - name: metrics
    port: 9090
    targetPort: 9090
  type: LoadBalancer

---
apiVersion: v1
kind: ConfigMap
metadata:
  name: mcp-config
data:
  production.toml: |
    [server]
    name = "production-mcp-server"
    version = "1.0.0"
    bind_address = "0.0.0.0"
    port = 8443

    [security]
    max_request_size = 1048576
    rate_limit_per_minute = 1000
    allowed_origins = ["https://yourdomain.com"]

    [monitoring]
    metrics_port = 9090
    health_check_port = 8080
    log_level = "info"
```

## Monitoring Setup

### 1. Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "mcp-alerts.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093

scrape_configs:
  - job_name: 'mcp-server'
    static_configs:
      - targets: ['mcp-server:9090']
    metrics_path: /metrics
    scrape_interval: 5s

  - job_name: 'mcp-health'
    static_configs:
      - targets: ['mcp-server:8080']
    metrics_path: /health
    scrape_interval: 10s
```

### 2. Alerting Rules

```yaml
# mcp-alerts.yml
groups:
- name: mcp-server-alerts
  rules:
  - alert: HighErrorRate
    expr: rate(mcp_errors_total[5m]) > 0.1
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "High error rate in MCP server"
      description: "Error rate is {{ $value }} errors per second"

  - alert: HighResponseTime
    expr: histogram_quantile(0.95, rate(mcp_request_duration_seconds_bucket[5m])) > 1
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "High response time in MCP server"
      description: "95th percentile response time is {{ $value }} seconds"

  - alert: HighMemoryUsage
    expr: (process_resident_memory_bytes / 1024 / 1024) > 800
    for: 15m
    labels:
      severity: warning
    annotations:
      summary: "High memory usage in MCP server"
      description: "Memory usage is {{ $value }} MB"

  - alert: ServiceDown
    expr: up{job="mcp-server"} == 0
    for: 1m
    labels:
      severity: critical
    annotations:
      summary: "MCP server is down"
      description: "MCP server instance {{ $labels.instance }} is down"
```

## Performance Optimization

### 1. Connection Pooling

```rust
use deadpool_postgres::{Config, Pool, Runtime};

pub async fn create_database_pool(database_url: &str) -> anyhow::Result<Pool> {
    let mut cfg = Config::new();
    cfg.url = Some(database_url.to_string());
    cfg.pool = Some(deadpool_postgres::PoolConfig {
        max_size: 20,
        timeouts: deadpool_postgres::Timeouts {
            wait: Some(std::time::Duration::from_secs(30)),
            create: Some(std::time::Duration::from_secs(30)),
            recycle: Some(std::time::Duration::from_secs(30)),
        },
    });

    cfg.create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
        .map_err(|e| anyhow::anyhow!("Failed to create database pool: {}", e))
}
```

### 2. Caching Layer

```rust
use mocopr_server::cache::*;

pub struct CacheConfig {
    pub redis_url: String,
    pub ttl_seconds: u64,
    pub max_memory_mb: u64,
}

pub async fn setup_cache(config: CacheConfig) -> anyhow::Result<Cache> {
    let redis_cache = RedisCache::new(&config.redis_url).await?;
    let memory_cache = LruCache::new(config.max_memory_mb * 1024 * 1024);

    Ok(Cache::layered(memory_cache, redis_cache)
        .with_default_ttl(std::time::Duration::from_secs(config.ttl_seconds)))
}
```

## Security Best Practices

### 1. Input Validation

```rust
use validator::{Validate, ValidationError};

#[derive(Debug, Validate)]
pub struct ToolCallRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    #[validate(custom = "validate_json_size")]
    pub arguments: serde_json::Value,
}

fn validate_json_size(value: &serde_json::Value) -> Result<(), ValidationError> {
    let serialized = serde_json::to_string(value)
        .map_err(|_| ValidationError::new("invalid_json"))?;

    if serialized.len() > 1024 * 1024 { // 1MB limit
        return Err(ValidationError::new("json_too_large"));
    }

    Ok(())
}
```

### 2. Rate Limiting

```rust
use governor::{Quota, RateLimiter};
use nonzero::NonZeroU32;

pub fn create_rate_limiter() -> RateLimiter<governor::state::direct::NotKeyed,
                                              governor::state::InMemoryState,
                                              governor::clock::DefaultClock> {
    RateLimiter::direct(Quota::per_minute(
        NonZeroU32::new(1000).unwrap()
    ))
}
```

## Backup and Recovery

### 1. Database Backup Strategy

```bash
#!/bin/bash
# backup.sh
set -e

BACKUP_DIR="/backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
DATABASE_URL=${DATABASE_URL}

# Create backup
pg_dump "${DATABASE_URL}" > "${BACKUP_DIR}/mcp_backup_${TIMESTAMP}.sql"

# Compress backup
gzip "${BACKUP_DIR}/mcp_backup_${TIMESTAMP}.sql"

# Upload to cloud storage (S3, GCS, etc.)
aws s3 cp "${BACKUP_DIR}/mcp_backup_${TIMESTAMP}.sql.gz" s3://your-backup-bucket/

# Clean up old backups (keep last 30 days)
find "${BACKUP_DIR}" -name "mcp_backup_*.sql.gz" -mtime +30 -delete

echo "Backup completed: mcp_backup_${TIMESTAMP}.sql.gz"
```

### 2. Disaster Recovery Plan

1. **Data Recovery**: Restore from latest backup
2. **Service Recovery**: Deploy from container registry
3. **Configuration Recovery**: Load from version control
4. **Monitoring**: Verify all systems operational

## Maintenance and Updates

### 1. Blue-Green Deployment

```bash
#!/bin/bash
# deploy.sh
set -e

NEW_VERSION=$1
CURRENT_COLOR=$(kubectl get service mcp-server -o jsonpath='{.spec.selector.color}')
NEW_COLOR=$([ "$CURRENT_COLOR" = "blue" ] && echo "green" || echo "blue")

echo "Current deployment: $CURRENT_COLOR"
echo "Deploying version $NEW_VERSION to: $NEW_COLOR"

# Deploy new version
kubectl set image deployment/mcp-server-$NEW_COLOR mcp-server=mcp-server:$NEW_VERSION
kubectl rollout status deployment/mcp-server-$NEW_COLOR

# Health check
kubectl wait --for=condition=ready pod -l app=mcp-server,color=$NEW_COLOR

# Switch traffic
kubectl patch service mcp-server -p '{"spec":{"selector":{"color":"'$NEW_COLOR'"}}}'

echo "Deployment completed. Traffic switched to $NEW_COLOR"
```

## Troubleshooting

### Common Issues

1. **High Memory Usage**: Check for memory leaks, optimize caching
2. **Connection Timeouts**: Adjust connection pool settings
3. **High Error Rates**: Check logs, validate inputs
4. **Performance Issues**: Profile and optimize hot paths

### Debugging Tools

```bash
# View logs
kubectl logs -f deployment/mcp-server

# Check metrics
curl http://mcp-server:9090/metrics

# Health check
curl http://mcp-server:8080/health

# Database connections
kubectl exec -it deployment/mcp-server -- netstat -an | grep 5432
```

## Summary and Current State

### What's Available Now

- **Complete MCP Protocol**: Full MCP specification implementation with all message types
- **Multiple Transport Protocols**:
  - **Stdio**: Production-ready process communication
  - **HTTP**: RESTful API endpoint (`/mcp`)
  - **WebSocket**: Real-time bidirectional communication (`/mcp/ws`)
- **Comprehensive Middleware System**:
  - `LoggingMiddleware`: Request/response logging with timing
  - `RateLimitMiddleware`: Configurable rate limiting
  - `AuthMiddleware`: API key and JWT authentication
  - `MetricsMiddleware`: Performance tracking
- **Built-in Monitoring**:
  - `PerformanceMetrics` with percentiles and resource tracking
  - `HealthCheck` system with detailed reporting
  - `MonitoringSystem` with periodic health checks
- **Flexible Server Builder**: Fluent API for complete configuration
- **Production-Ready Features**: Error handling, logging, security, validation

### Production-Ready Today

MoCoPr is production-ready now with all enterprise features implemented:

- Use stdio transport with process management (systemd, Docker, etc.)
- Implement monitoring at the infrastructure level
- Use reverse proxies (nginx, Envoy) for TLS and load balancing
- Handle authentication/authorization in your application logic
- Use external tools for metrics collection and alerting

**Example Production Setup:**

```text
Client → nginx (TLS) → Process Manager → MCP Server (stdio)
                    → Prometheus (metrics)
                    → Log Aggregation
```

This approach provides a robust production deployment while we continue building advanced features into MoCoPr itself.
