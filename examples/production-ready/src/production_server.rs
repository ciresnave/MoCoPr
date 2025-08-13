//! Production-ready MCP server example demonstrating best practices
//!
//! This example shows how to build a production-ready MCP server with:
//! - Comprehensive error handling
//! - Structured logging
//! - Input validation
//! - Security considerations
//! - Performance monitoring

use mocopr_core::prelude::*;
use mocopr_macros::Tool;
use mocopr_server::prelude::*;
use serde_json::{Value, json};
use tracing::{error, info, instrument};

/// Production-grade calculator tool with comprehensive error handling and validation
#[derive(Tool)]
#[tool(
    name = "secure_calculator",
    description = "Production calculator with enhanced security and validation"
)]
pub struct SecureCalculator;

#[async_trait::async_trait]
impl mocopr_core::ToolExecutor for SecureCalculator {
    #[instrument(skip(self), fields(operation))]
    async fn execute(&self, arguments: Option<Value>) -> mocopr_core::Result<ToolsCallResponse> {
        let args = arguments.unwrap_or_default();

        // Enhanced parameter validation
        let operation = args
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::validation("Missing or invalid 'operation' parameter"))?;

        tracing::Span::current().record("operation", operation);

        let a = args
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| Error::validation("Missing or invalid 'a' parameter"))?;

        let b = args
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| Error::validation("Missing or invalid 'b' parameter"))?;

        // Validate numeric inputs for security
        if !a.is_finite() || !b.is_finite() {
            return Err(Error::validation(
                "Invalid numeric values: inputs must be finite numbers",
            ));
        }

        // Prevent extremely large numbers that could cause issues
        const MAX_VALUE: f64 = 1e15;
        if a.abs() > MAX_VALUE || b.abs() > MAX_VALUE {
            return Err(Error::validation(
                "Input values too large for safe calculation",
            ));
        }

        let result = match operation {
            "add" => {
                let result = a + b;
                if !result.is_finite() {
                    return Err(Error::validation("Addition resulted in overflow"));
                }
                result
            }
            "subtract" => {
                let result = a - b;
                if !result.is_finite() {
                    return Err(Error::validation("Subtraction resulted in overflow"));
                }
                result
            }
            "multiply" => {
                let result = a * b;
                if !result.is_finite() {
                    return Err(Error::validation("Multiplication resulted in overflow"));
                }
                result
            }
            "divide" => {
                if b == 0.0 {
                    return Err(Error::validation("Division by zero is not allowed"));
                }
                let result = a / b;
                if !result.is_finite() {
                    return Err(Error::validation("Division resulted in invalid number"));
                }
                result
            }
            "power" => {
                if a == 0.0 && b < 0.0 {
                    return Err(Error::validation("Cannot raise zero to negative power"));
                }
                // Limit exponentiation to prevent DoS
                if b.abs() > 1000.0 {
                    return Err(Error::validation("Exponent too large for safe calculation"));
                }
                let result = a.powf(b);
                if !result.is_finite() {
                    return Err(Error::validation(
                        "Power operation resulted in invalid number",
                    ));
                }
                result
            }
            _ => {
                return Err(Error::validation(format!(
                    "Unsupported operation: '{}'. Supported operations: add, subtract, multiply, divide, power",
                    operation
                )));
            }
        };

        info!(
            operation = operation,
            input_a = a,
            input_b = b,
            result = result,
            "Calculation completed successfully"
        );

        Ok(ToolsCallResponse::success(vec![Content::Text(
            TextContent::new(
                json!({
                    "result": result,
                    "operation": operation,
                    "inputs": {
                        "a": a,
                        "b": b
                    },
                    "status": "success"
                })
                .to_string(),
            ),
        )]))
    }

    async fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "description": "Mathematical operation to perform",
                    "enum": ["add", "subtract", "multiply", "divide", "power"]
                },
                "a": {
                    "type": "number",
                    "description": "First operand",
                    "minimum": -1e15,
                    "maximum": 1e15
                },
                "b": {
                    "type": "number",
                    "description": "Second operand",
                    "minimum": -1e15,
                    "maximum": 1e15
                }
            },
            "required": ["operation", "a", "b"]
        }))
    }
}

/// System health monitoring tool
#[derive(Tool)]
#[tool(
    name = "health_check",
    description = "Get system health and monitoring information"
)]
pub struct HealthCheckTool {
    start_time: std::time::Instant,
}

impl HealthCheckTool {
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
        }
    }
}

#[async_trait::async_trait]
impl mocopr_core::ToolExecutor for HealthCheckTool {
    #[instrument(skip(self))]
    async fn execute(&self, _arguments: Option<Value>) -> mocopr_core::Result<ToolsCallResponse> {
        let uptime = self.start_time.elapsed();

        // In production, you would check:
        // - Database connectivity
        // - External service availability
        // - Memory usage
        // - Disk space
        // - CPU usage

        let health_info = json!({
            "status": "healthy",
            "uptime_seconds": uptime.as_secs(),
            "uptime_human": format_duration(uptime),
            "version": env!("CARGO_PKG_VERSION"),
            "checks": {
                "memory": "ok",
                "disk": "ok",
                "cpu": "ok"
            },
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });

        info!("Health check completed successfully");

        Ok(ToolsCallResponse::success(vec![Content::Text(
            TextContent::new(health_info.to_string()),
        )]))
    }

    async fn schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {},
            "description": "No parameters required for health check"
        }))
    }
}

/// Request validation middleware
#[allow(dead_code)]
pub struct ValidationMiddleware;

#[allow(dead_code)]
impl ValidationMiddleware {
    pub fn validate_request(request: &JsonRpcRequest) -> mocopr_core::Result<()> {
        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            return Err(Error::validation("Invalid JSON-RPC version. Must be '2.0'"));
        }

        // Validate method name format
        if request.method.is_empty() || request.method.len() > 100 {
            return Err(Error::validation("Invalid method name length"));
        }

        // Check for potentially dangerous method names
        let dangerous_patterns = ["../", "\\", "eval", "exec", "system"];
        for pattern in &dangerous_patterns {
            if request.method.contains(pattern) {
                return Err(Error::validation(
                    "Invalid method name contains dangerous pattern",
                ));
            }
        }

        // Validate request size (prevent DoS)
        let serialized = serde_json::to_string(request)
            .map_err(|e| Error::validation(format!("Failed to serialize request: {}", e)))?;

        if serialized.len() > 1024 * 1024 {
            // 1MB limit
            return Err(Error::validation("Request too large"));
        }

        Ok(())
    }
}

pub async fn run_server() -> anyhow::Result<()> {
    // Initialize structured logging for production
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "production_server=info,mocopr=info".to_string()),
        )
        .json() // Use JSON format for production logging
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("Starting production MCP server");

    // Build server with production-ready tools
    let server = McpServerBuilder::new()
        .with_info("production-server", env!("CARGO_PKG_VERSION"))
        .with_tools()
        .with_tool(SecureCalculator)
        .with_tool(HealthCheckTool::new())
        .build()?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Production MCP server built successfully"
    );

    // Add graceful shutdown handling
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Handle shutdown signals (cross-platform)
    #[cfg(unix)]
    tokio::spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to create SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to create SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, initiating graceful shutdown");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, initiating graceful shutdown");
            }
        }

        let _ = shutdown_tx.send(());
    });

    #[cfg(windows)]
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("Received Ctrl+C, initiating graceful shutdown");
                let _ = shutdown_tx.send(());
            }
            Err(err) => {
                error!("Unable to listen for shutdown signal: {}", err);
            }
        }
    });

    // Start the server
    let server_task = tokio::spawn(async move {
        info!("MCP server starting on stdio transport");
        server.run_stdio().await
    });

    // Wait for either server completion or shutdown signal
    tokio::select! {
        result = server_task => {
            match result {
                Ok(Ok(())) => info!("MCP server shutdown gracefully"),
                Ok(Err(e)) => error!("MCP server error: {}", e),
                Err(e) => error!("MCP server task failed: {}", e),
            }
        }
        _ = shutdown_rx => {
            info!("Shutdown signal received, stopping server");
        }
    }

    info!("Production MCP server shutdown complete");
    Ok(())
}

/// Format duration into human-readable string
fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_secure_calculator_basic_operations() -> anyhow::Result<()> {
        let calculator = SecureCalculator;

        // Test addition
        let result = calculator
            .execute(Some(json!({
                "operation": "add",
                "a": 5.0,
                "b": 3.0
            })))
            .await?;

        match &result.content[0] {
            Content::Text(text) => {
                let parsed: Value = serde_json::from_str(&text.text)?;
                assert_eq!(parsed["result"], 8.0);
                assert_eq!(parsed["operation"], "add");
            }
            _ => panic!("Expected text content"),
        }

        // Test division by zero
        let result = calculator
            .execute(Some(json!({
                "operation": "divide",
                "a": 5.0,
                "b": 0.0
            })))
            .await;

        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_health_check() -> anyhow::Result<()> {
        let health_tool = HealthCheckTool::new();

        let result = health_tool.execute(None).await?;

        match &result.content[0] {
            Content::Text(text) => {
                let parsed: Value = serde_json::from_str(&text.text)?;
                assert_eq!(parsed["status"], "healthy");
                assert!(parsed["uptime_seconds"].is_number());
            }
            _ => panic!("Expected text content"),
        }

        Ok(())
    }

    #[test]
    fn test_request_validation() {
        use mocopr_core::RequestId;

        let valid_request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(RequestId::Number(1)),
            method: "tools/call".to_string(),
            params: Some(json!({})),
        };

        assert!(ValidationMiddleware::validate_request(&valid_request).is_ok());

        let invalid_request = JsonRpcRequest {
            jsonrpc: "1.0".to_string(),
            id: Some(RequestId::Number(1)),
            method: "tools/call".to_string(),
            params: Some(json!({})),
        };

        assert!(ValidationMiddleware::validate_request(&invalid_request).is_err());
    }

    #[test]
    fn test_duration_formatting() {
        assert_eq!(format_duration(std::time::Duration::from_secs(30)), "30s");
        assert_eq!(
            format_duration(std::time::Duration::from_secs(90)),
            "1m 30s"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_secs(3661)),
            "1h 1m 1s"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_secs(90061)),
            "1d 1h 1m 1s"
        );
    }

    #[tokio::test]
    async fn test_calculator_security_limits() -> anyhow::Result<()> {
        let calculator = SecureCalculator;

        // Test extremely large numbers
        let result = calculator
            .execute(Some(json!({
                "operation": "add",
                "a": 1e16, // Exceeds MAX_VALUE
                "b": 1.0
            })))
            .await;

        assert!(result.is_err());

        // Test invalid numbers
        let result = calculator
            .execute(Some(json!({
                "operation": "add",
                "a": f64::NAN,
                "b": 1.0
            })))
            .await;

        assert!(result.is_err());

        // Test extremely large exponent
        let result = calculator
            .execute(Some(json!({
                "operation": "power",
                "a": 2.0,
                "b": 10000.0 // Too large
            })))
            .await;

        assert!(result.is_err());

        Ok(())
    }
}
