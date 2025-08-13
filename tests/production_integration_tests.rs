// Production-ready integration tests with actual security and error recovery
// This replaces the mock integration tests with real implementations

use anyhow::Result;
use mocopr_core::ResourceReader;
use mocopr_core::monitoring::{
    BasicHealthCheck, FileSystemHealthCheck, HealthReport, MonitoringConfig, MonitoringSystem,
    PerformanceMetrics, RequestMetrics,
};
use mocopr_core::prelude::*;
use mocopr_core::security::{ErrorRecoverySystem, SecurityValidator};
use mocopr_macros::Resource;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::time::timeout;
use tracing::info;
use url::Url;

/// Test configuration
const TEST_TIMEOUT: Duration = Duration::from_secs(30);
const SECURITY_TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Secure file resource with actual security validation
#[derive(Resource)]
#[resource(
    name = "secure_file",
    description = "Secure file resource with path validation"
)]
struct SecureFileResource {
    root_dir: PathBuf,
    security_validator: SecurityValidator,
}

impl SecureFileResource {
    fn new(root_dir: PathBuf) -> Self {
        let security_validator = SecurityValidator::new()
            .with_root_directory(root_dir.clone())
            .with_allowed_extensions(vec![
                "txt".to_string(),
                "md".to_string(),
                "json".to_string(),
            ])
            .with_max_file_size(1024 * 1024); // 1MB limit

        Self {
            root_dir,
            security_validator,
        }
    }

    async fn read_file(&self, uri: &str) -> Result<String> {
        // Parse URI
        let parsed_uri =
            Url::parse(uri).map_err(|e| Error::validation(format!("Invalid URI: {}", e)))?;

        // Security validation
        self.security_validator
            .validate_resource_access(&parsed_uri)?;

        // Convert to file path
        let file_path = parsed_uri
            .to_file_path()
            .map_err(|_| Error::validation("Invalid file path in URI"))?;

        // Additional security check
        self.security_validator.validate_file_path_buf(&file_path)?;

        // Ensure file is within root directory (defense in depth)
        if !file_path.starts_with(&self.root_dir) {
            return Err(anyhow::anyhow!(
                "File path is outside the configured root directory"
            ));
        }

        // Read file content
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| Error::resource_error(format!("Failed to read file: {}", e)))?;

        Ok(content)
    }
}

#[async_trait::async_trait]
impl ResourceReader for SecureFileResource {
    async fn read_resource(&self) -> mocopr_core::Result<Vec<ResourceContent>> {
        // For the test, we'll read a default file or return mock content
        // In a real implementation, this would be more sophisticated
        let mock_uri = Url::parse("file:///test/mock.txt")
            .map_err(|e| Error::validation(format!("Failed to create mock URI: {}", e)))?;

        let content = vec![Content::Text(TextContent::new("Mock secure file content"))];

        Ok(vec![ResourceContent::new(mock_uri, content)])
    }
}

/// Test server with security and monitoring
struct TestServer {
    monitoring: MonitoringSystem,
    error_recovery: ErrorRecoverySystem,
    secure_resource: SecureFileResource,
}

impl TestServer {
    async fn new(root_dir: PathBuf) -> Self {
        let monitoring_config = MonitoringConfig {
            max_response_times: 1000,
            health_check_interval: Duration::from_secs(10),
            detailed_logging: true,
        };

        let monitoring = MonitoringSystem::new(monitoring_config);
        let error_recovery = ErrorRecoverySystem::new();
        let secure_resource = SecureFileResource::new(root_dir.clone());

        // Register health checks
        monitoring
            .register_health_check(Box::new(BasicHealthCheck::new("server".to_string())))
            .await;
        monitoring
            .register_health_check(Box::new(FileSystemHealthCheck::new(root_dir)))
            .await;

        Self {
            monitoring,
            error_recovery,
            secure_resource,
        }
    }

    async fn handle_request(&self, method: &str, params: Value) -> Result<Value> {
        let start_time = Instant::now();
        let mut success = false;
        let mut error_message = None;

        let result = match method {
            "resources/read" => {
                if let Some(uri) = params.get("uri").and_then(|v| v.as_str()) {
                    match self.secure_resource.read_file(uri).await {
                        Ok(content) => {
                            success = true;
                            Ok(json!({ "contents": [{ "text": content }] }))
                        }
                        Err(e) => {
                            error_message = Some(e.to_string());
                            Err(e)
                        }
                    }
                } else {
                    let err = self
                        .error_recovery
                        .handle_invalid_parameters(method, "Missing 'uri' parameter");
                    error_message = Some(err.to_string());
                    Err(err.into())
                }
            }
            "tools/call" => {
                if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
                    match name {
                        "valid_tool" => {
                            success = true;
                            Ok(
                                json!({ "content": [{ "type": "text", "text": "Tool executed successfully" }] }),
                            )
                        }
                        "nonexistent_tool" => {
                            let err = self.error_recovery.handle_invalid_method(name);
                            error_message = Some(err.to_string());
                            Err(err.into())
                        }
                        _ => {
                            let err = self.error_recovery.handle_invalid_method(name);
                            error_message = Some(err.to_string());
                            Err(err.into())
                        }
                    }
                } else {
                    let err = self
                        .error_recovery
                        .handle_invalid_parameters(method, "Missing 'name' parameter");
                    error_message = Some(err.to_string());
                    Err(err.into())
                }
            }
            _ => {
                let err = self.error_recovery.handle_invalid_method(method);
                error_message = Some(err.to_string());
                Err(err.into())
            }
        };

        // Record request metrics
        let request_metrics = RequestMetrics {
            start_time,
            method: method.to_string(),
            success,
            response_time: start_time.elapsed(),
            error_message,
        };

        self.monitoring.record_request(request_metrics).await;

        result
    }

    async fn get_health_report(&self) -> HealthReport {
        self.monitoring.health_check().await
    }

    async fn get_metrics(&self) -> PerformanceMetrics {
        self.monitoring.get_metrics().await
    }
}

/// Test client for making requests
struct TestClient {
    server: TestServer,
}

impl TestClient {
    fn new(server: TestServer) -> Self {
        Self { server }
    }

    async fn read_resource(&self, uri: &str) -> Result<Value> {
        self.server
            .handle_request("resources/read", json!({ "uri": uri }))
            .await
    }

    async fn call_tool(&self, name: &str, params: Value) -> Result<Value> {
        self.server
            .handle_request("tools/call", json!({ "name": name, "arguments": params }))
            .await
    }
}

#[tokio::test]
async fn test_security_path_traversal_protection() -> Result<()> {
    let test_result = timeout(SECURITY_TEST_TIMEOUT, async {
        // Create temporary directory with test files
        let temp_dir = TempDir::new()?;
        let safe_dir = temp_dir.path().join("safe");
        tokio::fs::create_dir_all(&safe_dir).await?;

        // Create test files
        let safe_file = safe_dir.join("test.txt");
        tokio::fs::write(&safe_file, "Safe content").await?;

        // Create server with security validation
        let server = TestServer::new(safe_dir.clone()).await;
        let client = TestClient::new(server);

        // Test legitimate file access
        let safe_uri = format!("file://{}", safe_file.display());
        let result = client.read_resource(&safe_uri).await;
        assert!(result.is_ok(), "Should allow access to safe file");

        // Test path traversal attempts
        let malicious_uris = vec![
            format!("file://{}", safe_dir.join("../../../etc/passwd").display()),
            format!("file://{}", safe_dir.join("../../sensitive.txt").display()),
            format!("file://{}", safe_dir.join("../outside/file.txt").display()),
        ];

        for malicious_uri in malicious_uris {
            let result = client.read_resource(&malicious_uri).await;
            assert!(
                result.is_err(),
                "Should reject path traversal: {}",
                malicious_uri
            );

            // Verify it's a security error
            if let Err(e) = result {
                let error_str = e.to_string();
                assert!(
                    error_str.contains("outside of allowed directory")
                        || error_str.contains("security")
                        || error_str.contains("Security error")
                        || error_str.contains("path traversal")
                        || error_str.contains("canonicalize")
                        || error_str.contains("not allowed"),
                    "Should be a security error: {}",
                    error_str
                );
            }
        }

        info!("✅ Path traversal protection test passed");
        Ok::<(), anyhow::Error>(())
    })
    .await;

    test_result??;
    Ok(())
}

#[tokio::test]
async fn test_error_recovery_and_resilience() -> Result<()> {
    let test_result = timeout(TEST_TIMEOUT, async {
        let temp_dir = TempDir::new()?;
        let server = TestServer::new(temp_dir.path().to_path_buf()).await;
        let client = TestClient::new(server);

        // Test invalid method calls
        let invalid_method_result = client.call_tool("nonexistent_tool", json!({})).await;
        assert!(
            invalid_method_result.is_err(),
            "Should reject invalid method"
        );

        if let Err(e) = invalid_method_result {
            let error_str = e.to_string();
            assert!(
                error_str.contains("not supported")
                    || error_str.contains("not found")
                    || error_str.contains("invalid"),
                "Should be a method not found error: {}",
                error_str
            );
        }

        // Test malformed parameters
        let _invalid_params_result = client
            .call_tool(
                "valid_tool",
                json!({
                    "invalid": "parameters"
                }),
            )
            .await;
        // This should actually succeed as the valid_tool doesn't validate parameters strictly

        // Test missing required parameters
        let missing_params = client.read_resource("").await;
        assert!(missing_params.is_err(), "Should reject empty URI");

        // Test that valid operations still work after errors
        let valid_result = client.call_tool("valid_tool", json!({})).await;
        assert!(
            valid_result.is_ok(),
            "Should handle valid requests after errors"
        );

        info!("✅ Error recovery test passed");
        Ok::<(), anyhow::Error>(())
    })
    .await;

    test_result??;
    Ok(())
}

#[tokio::test]
async fn test_monitoring_and_health_checks() -> Result<()> {
    let test_result = timeout(TEST_TIMEOUT, async {
        let temp_dir = TempDir::new()?;
        let server = TestServer::new(temp_dir.path().to_path_buf()).await;
        let client = TestClient::new(server);

        // Make some requests to generate metrics
        let _ = client.call_tool("valid_tool", json!({})).await;
        let _ = client.call_tool("nonexistent_tool", json!({})).await;

        // Check health report
        let health_report = client.server.get_health_report().await;
        assert!(
            !health_report.checks.is_empty(),
            "Should have health checks"
        );

        // Check metrics
        let metrics = client.server.get_metrics().await;
        assert!(metrics.total_requests > 0, "Should have recorded requests");
        assert!(
            metrics.successful_requests > 0,
            "Should have successful requests"
        );
        assert!(metrics.failed_requests > 0, "Should have failed requests");

        info!("✅ Monitoring and health check test passed");
        info!("Health status: {:?}", health_report.status);
        info!("Total requests: {}", metrics.total_requests);
        info!(
            "Success rate: {:.2}%",
            (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0
        );

        Ok::<(), anyhow::Error>(())
    })
    .await;

    test_result??;
    Ok(())
}

#[tokio::test]
async fn test_file_extension_validation() -> Result<()> {
    let test_result = timeout(SECURITY_TEST_TIMEOUT, async {
        let temp_dir = TempDir::new()?;
        let safe_dir = temp_dir.path().join("safe");
        tokio::fs::create_dir_all(&safe_dir).await?;

        // Create files with different extensions
        let allowed_file = safe_dir.join("test.txt");
        let disallowed_file = safe_dir.join("test.exe");

        tokio::fs::write(&allowed_file, "Safe content").await?;
        tokio::fs::write(&disallowed_file, "Executable content").await?;

        let server = TestServer::new(safe_dir.clone()).await;
        let client = TestClient::new(server);

        // Test allowed extension
        let safe_uri = format!("file://{}", allowed_file.display());
        let result = client.read_resource(&safe_uri).await;
        assert!(result.is_ok(), "Should allow .txt files");

        // Test disallowed extension
        let unsafe_uri = format!("file://{}", disallowed_file.display());
        let result = client.read_resource(&unsafe_uri).await;
        assert!(result.is_err(), "Should reject .exe files");

        info!("✅ File extension validation test passed");
        Ok::<(), anyhow::Error>(())
    })
    .await;

    test_result??;
    Ok(())
}

#[tokio::test]
async fn test_file_size_limits() -> Result<()> {
    let test_result = timeout(SECURITY_TEST_TIMEOUT, async {
        let temp_dir = TempDir::new()?;
        let safe_dir = temp_dir.path().join("safe");
        tokio::fs::create_dir_all(&safe_dir).await?;

        // Create a large file (2MB, exceeding 1MB limit)
        let large_file = safe_dir.join("large.txt");
        let large_content = "x".repeat(2 * 1024 * 1024); // 2MB
        tokio::fs::write(&large_file, large_content).await?;

        let server = TestServer::new(safe_dir.clone()).await;
        let client = TestClient::new(server);

        // Test file size limit
        let large_uri = format!("file://{}", large_file.display());
        let result = client.read_resource(&large_uri).await;
        assert!(result.is_err(), "Should reject files exceeding size limit");

        if let Err(e) = result {
            let error_str = e.to_string();
            assert!(
                error_str.contains("size") || error_str.contains("limit"),
                "Should be a size limit error: {}",
                error_str
            );
        }

        info!("✅ File size limit test passed");
        Ok::<(), anyhow::Error>(())
    })
    .await;

    test_result??;
    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let test_result = timeout(TEST_TIMEOUT, async {
        let temp_dir = TempDir::new()?;
        let safe_dir = temp_dir.path().join("safe");
        tokio::fs::create_dir_all(&safe_dir).await?;

        // Create test file
        let test_file = safe_dir.join("test.txt");
        tokio::fs::write(&test_file, "Test content").await?;

        let server = TestServer::new(safe_dir.clone()).await;

        // Run concurrent operations
        let test_uri = format!("file://{}", test_file.display());
        let mut tasks = Vec::new();

        // Create shared client wrapped in Arc
        let client = std::sync::Arc::new(TestClient::new(server));

        for i in 0..10 {
            let client = client.clone();
            let uri = test_uri.clone();

            let task = tokio::spawn(async move {
                if i % 2 == 0 {
                    client.read_resource(&uri).await
                } else {
                    client.call_tool("valid_tool", json!({})).await
                }
            });

            tasks.push(task);
        }

        // Wait for all tasks to complete
        let mut successful = 0;
        let mut failed = 0;

        for task in tasks {
            match task.await? {
                Ok(_) => successful += 1,
                Err(_) => failed += 1,
            }
        }

        assert!(successful > 0, "Should have some successful operations");

        // Check final metrics
        let metrics = client.server.get_metrics().await;
        assert!(
            metrics.total_requests >= 10,
            "Should have recorded all requests"
        );

        info!("✅ Concurrent operations test passed");
        info!(
            "Successful operations: {}, Failed operations: {}",
            successful, failed
        );

        Ok::<(), anyhow::Error>(())
    })
    .await;

    test_result??;
    Ok(())
}

#[tokio::test]
async fn test_comprehensive_security_audit() -> Result<()> {
    let test_result = timeout(TEST_TIMEOUT, async {
        let temp_dir = TempDir::new()?;
        let safe_dir = temp_dir.path().join("safe");
        tokio::fs::create_dir_all(&safe_dir).await?;

        let server = TestServer::new(safe_dir.clone()).await;
        let client = TestClient::new(server);

        // Test various security scenarios
        let security_tests = vec![
            // Path traversal attempts
            ("file:///../../../etc/passwd", "path traversal"),
            ("file:///../../sensitive.txt", "path traversal"),
            ("file://../outside/file.txt", "path traversal"),
            // Invalid URIs
            ("invalid://uri", "invalid scheme"),
            ("file://", "empty path"),
            // Malformed URIs
            ("not_a_uri", "malformed URI"),
        ];

        for (uri, test_name) in security_tests {
            let result = client.read_resource(uri).await;
            assert!(
                result.is_err(),
                "Security test '{}' should fail for URI: {}",
                test_name,
                uri
            );

            if let Err(e) = result {
                let error_str = e.to_string();
                info!(
                    "Security test '{}' correctly rejected: {}",
                    test_name, error_str
                );
            }
        }

        info!("✅ Comprehensive security audit test passed");
        Ok::<(), anyhow::Error>(())
    })
    .await;

    test_result??;
    Ok(())
}

#[tokio::test]
async fn test_stress_and_performance() -> Result<()> {
    let test_result = timeout(TEST_TIMEOUT, async {
        let temp_dir = TempDir::new()?;
        let safe_dir = temp_dir.path().join("safe");
        tokio::fs::create_dir_all(&safe_dir).await?;

        // Create test file
        let test_file = safe_dir.join("test.txt");
        tokio::fs::write(&test_file, "Test content").await?;

        let server = TestServer::new(safe_dir.clone()).await;
        let client = TestClient::new(server);

        // Stress test with many rapid requests
        let start_time = Instant::now();
        let test_uri = format!("file://{}", test_file.display());

        for i in 0..100 {
            let _result = if i % 3 == 0 {
                client.read_resource(&test_uri).await
            } else {
                client.call_tool("valid_tool", json!({})).await
            };

            // Allow some failures due to rate limiting or other factors
            if i % 10 == 0 {
                info!("Completed {} requests", i + 1);
            }
        }

        let total_time = start_time.elapsed();
        let metrics = client.server.get_metrics().await;

        info!("✅ Stress test completed in {:?}", total_time);
        info!("Total requests: {}", metrics.total_requests);
        info!(
            "Average response time: {:.2}ms",
            metrics.avg_response_time_ms
        );
        info!("95th percentile: {:.2}ms", metrics.p95_response_time_ms);
        info!("99th percentile: {:.2}ms", metrics.p99_response_time_ms);

        assert!(
            metrics.total_requests >= 100,
            "Should have processed all requests"
        );

        Ok::<(), anyhow::Error>(())
    })
    .await;

    test_result??;
    Ok(())
}
