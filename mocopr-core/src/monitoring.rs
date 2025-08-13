// Comprehensive monitoring and observability system for MoCoPr
// This provides production-ready monitoring capabilities

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

/// Health check status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthStatus {
    /// System is healthy
    Healthy,
    /// System is degraded but operational
    Degraded,
    /// System is unhealthy and may not function correctly
    Unhealthy,
    /// System is in unknown state
    Unknown,
}

/// Individual health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Name of the health check
    pub name: String,
    /// Status of the health check
    pub status: HealthStatus,
    /// Optional message providing details
    pub message: Option<String>,
    /// Time when the check was performed
    pub timestamp: SystemTime,
    /// Duration the check took to complete
    pub duration: Duration,
}

/// Overall system health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Overall status (worst of all individual checks)
    pub status: HealthStatus,
    /// Individual check results
    pub checks: Vec<HealthCheckResult>,
    /// Time when the report was generated
    pub timestamp: SystemTime,
    /// Total time to generate the report
    pub total_duration: Duration,
}

/// Trait for implementing health checks
#[async_trait::async_trait]
pub trait HealthCheck: Send + Sync {
    /// Name of the health check
    fn name(&self) -> &str;

    /// Perform the health check
    async fn check(&self) -> HealthCheckResult;
}

/// Performance metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total number of requests processed
    pub total_requests: u64,
    /// Number of successful requests
    pub successful_requests: u64,
    /// Number of failed requests
    pub failed_requests: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// 95th percentile response time in milliseconds
    pub p95_response_time_ms: f64,
    /// 99th percentile response time in milliseconds
    pub p99_response_time_ms: f64,
    /// Current active connections
    pub active_connections: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Timestamp when metrics were collected
    pub timestamp: SystemTime,
}

/// Request metrics for tracking individual operations
#[derive(Debug, Clone)]
pub struct RequestMetrics {
    /// Request start time
    pub start_time: Instant,
    /// Request method/operation
    pub method: String,
    /// Request success status
    pub success: bool,
    /// Response time
    pub response_time: Duration,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Comprehensive monitoring system
pub struct MonitoringSystem {
    /// Registered health checks
    health_checks: Arc<RwLock<Vec<Box<dyn HealthCheck>>>>,
    /// Performance metrics
    metrics: Arc<RwLock<PerformanceMetrics>>,
    /// Recent response times for percentile calculations
    response_times: Arc<RwLock<Vec<Duration>>>,
    /// Configuration
    config: MonitoringConfig,
}

/// Configuration for monitoring system
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    /// Maximum number of response times to keep in memory
    pub max_response_times: usize,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Enable detailed logging
    pub detailed_logging: bool,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            max_response_times: 10000,
            health_check_interval: Duration::from_secs(30),
            detailed_logging: true,
        }
    }
}

impl MonitoringSystem {
    /// Create a new monitoring system
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            health_checks: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            response_times: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Register a health check
    pub async fn register_health_check(&self, check: Box<dyn HealthCheck>) {
        let mut checks = self.health_checks.write().await;
        checks.push(check);
    }

    /// Run all health checks and generate a report
    pub async fn health_check(&self) -> HealthReport {
        let start_time = Instant::now();
        let mut results = Vec::new();
        let mut overall_status = HealthStatus::Healthy;

        let checks = self.health_checks.read().await;

        for check in checks.iter() {
            let result = check.check().await;

            // Update overall status (worst case)
            match (&overall_status, &result.status) {
                (HealthStatus::Healthy, HealthStatus::Degraded) => {
                    overall_status = HealthStatus::Degraded
                }
                (HealthStatus::Healthy | HealthStatus::Degraded, HealthStatus::Unhealthy) => {
                    overall_status = HealthStatus::Unhealthy
                }
                (
                    HealthStatus::Healthy | HealthStatus::Degraded | HealthStatus::Unhealthy,
                    HealthStatus::Unknown,
                ) => overall_status = HealthStatus::Unknown,
                _ => {}
            }

            results.push(result);
        }

        let total_duration = start_time.elapsed();

        HealthReport {
            status: overall_status,
            checks: results,
            timestamp: SystemTime::now(),
            total_duration,
        }
    }

    /// Record a request for metrics
    pub async fn record_request(&self, request: RequestMetrics) {
        let mut metrics = self.metrics.write().await;
        let mut response_times = self.response_times.write().await;

        // Update basic counters
        metrics.total_requests += 1;
        if request.success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }

        // Update response times
        response_times.push(request.response_time);

        // Keep only recent response times
        let current_len = response_times.len();
        if current_len > self.config.max_response_times {
            response_times.drain(0..current_len - self.config.max_response_times);
        }

        // Calculate percentiles
        let mut sorted_times = response_times.clone();
        sorted_times.sort();

        if !sorted_times.is_empty() {
            let avg_ms = sorted_times.iter().sum::<Duration>().as_secs_f64() * 1000.0
                / sorted_times.len() as f64;
            let p95_idx = (sorted_times.len() as f64 * 0.95) as usize;
            let p99_idx = (sorted_times.len() as f64 * 0.99) as usize;

            metrics.avg_response_time_ms = avg_ms;
            metrics.p95_response_time_ms = sorted_times
                .get(p95_idx)
                .unwrap_or(&Duration::ZERO)
                .as_secs_f64()
                * 1000.0;
            metrics.p99_response_time_ms = sorted_times
                .get(p99_idx)
                .unwrap_or(&Duration::ZERO)
                .as_secs_f64()
                * 1000.0;
        }

        // Update timestamp
        metrics.timestamp = SystemTime::now();

        // Log request if detailed logging is enabled
        if self.config.detailed_logging {
            if request.success {
                debug!(
                    "Request completed: {} in {:?}",
                    request.method, request.response_time
                );
            } else {
                warn!(
                    "Request failed: {} in {:?} - {}",
                    request.method,
                    request.response_time,
                    request
                        .error_message
                        .unwrap_or_else(|| "Unknown error".to_string())
                );
            }
        }
    }

    /// Get current performance metrics
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }

    /// Start periodic health checks
    pub async fn start_periodic_health_checks(&self) {
        let health_checks = self.health_checks.clone();
        let interval = self.config.health_check_interval;

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                let checks = health_checks.read().await;
                for check in checks.iter() {
                    let result = check.check().await;

                    match result.status {
                        HealthStatus::Healthy => {
                            debug!("Health check '{}' passed", result.name);
                        }
                        HealthStatus::Degraded => {
                            warn!(
                                "Health check '{}' degraded: {}",
                                result.name,
                                result.message.unwrap_or_else(|| "No details".to_string())
                            );
                        }
                        HealthStatus::Unhealthy => {
                            error!(
                                "Health check '{}' failed: {}",
                                result.name,
                                result.message.unwrap_or_else(|| "No details".to_string())
                            );
                        }
                        HealthStatus::Unknown => {
                            warn!(
                                "Health check '{}' status unknown: {}",
                                result.name,
                                result.message.unwrap_or_else(|| "No details".to_string())
                            );
                        }
                    }
                }
            }
        });
    }

    /// Update system resource metrics
    pub async fn update_system_metrics(&self, active_connections: u64) {
        let mut metrics = self.metrics.write().await;
        metrics.active_connections = active_connections;

        // Update system resource usage
        #[cfg(target_os = "linux")]
        {
            if let Ok(usage) = self.get_system_usage().await {
                metrics.memory_usage_bytes = usage.memory_bytes;
                metrics.cpu_usage_percent = usage.cpu_percent;
            }
        }
    }

    /// Get system resource usage (Linux only)
    #[cfg(target_os = "linux")]
    async fn get_system_usage(&self) -> Result<SystemUsage, Box<dyn std::error::Error>> {
        use std::fs;

        // Read memory usage from /proc/self/status
        let status = fs::read_to_string("/proc/self/status")?;
        let memory_kb = status
            .lines()
            .find(|line| line.starts_with("VmRSS:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        // Read CPU usage from /proc/self/stat
        let stat = fs::read_to_string("/proc/self/stat")?;
        let fields: Vec<&str> = stat.split_whitespace().collect();
        let utime = fields
            .get(13)
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        let stime = fields
            .get(14)
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        // Simple CPU usage calculation (this is a simplified version)
        let cpu_percent = ((utime + stime) as f64 / 100.0) * 0.1; // Rough estimate

        Ok(SystemUsage {
            memory_bytes: memory_kb * 1024,
            cpu_percent,
        })
    }
}

#[cfg(target_os = "linux")]
struct SystemUsage {
    memory_bytes: u64,
    cpu_percent: f64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time_ms: 0.0,
            p95_response_time_ms: 0.0,
            p99_response_time_ms: 0.0,
            active_connections: 0,
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
            timestamp: SystemTime::now(),
        }
    }
}

/// Built-in health check for basic system status
pub struct BasicHealthCheck {
    name: String,
}

impl BasicHealthCheck {
    /// Create a new HTTP health check
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[async_trait::async_trait]
impl HealthCheck for BasicHealthCheck {
    fn name(&self) -> &str {
        &self.name
    }

    async fn check(&self) -> HealthCheckResult {
        let start_time = Instant::now();

        // Basic system health check
        let status = if std::env::var("HEALTH_CHECK_FAIL").is_ok() {
            HealthStatus::Unhealthy
        } else {
            HealthStatus::Healthy
        };

        let duration = start_time.elapsed();

        HealthCheckResult {
            name: self.name.clone(),
            status,
            message: Some("Basic system health check".to_string()),
            timestamp: SystemTime::now(),
            duration,
        }
    }
}

/// Health check for file system access
pub struct FileSystemHealthCheck {
    test_path: std::path::PathBuf,
}

impl FileSystemHealthCheck {
    /// Create a new file health check
    pub fn new(test_path: std::path::PathBuf) -> Self {
        Self { test_path }
    }
}

#[async_trait::async_trait]
impl HealthCheck for FileSystemHealthCheck {
    fn name(&self) -> &str {
        "filesystem"
    }

    async fn check(&self) -> HealthCheckResult {
        let start_time = Instant::now();

        let (status, message) = match std::fs::metadata(&self.test_path) {
            Ok(_) => (HealthStatus::Healthy, "File system accessible".to_string()),
            Err(e) => (
                HealthStatus::Unhealthy,
                format!("File system check failed: {}", e),
            ),
        };

        let duration = start_time.elapsed();

        HealthCheckResult {
            name: "filesystem".to_string(),
            status,
            message: Some(message),
            timestamp: SystemTime::now(),
            duration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_monitoring_system() {
        let config = MonitoringConfig::default();
        let monitoring = MonitoringSystem::new(config);

        // Register a health check
        let health_check = Box::new(BasicHealthCheck::new("test".to_string()));
        monitoring.register_health_check(health_check).await;

        // Run health check
        let report = monitoring.health_check().await;
        assert_eq!(report.status, HealthStatus::Healthy);
        assert_eq!(report.checks.len(), 1);

        // Record a request
        let request = RequestMetrics {
            start_time: Instant::now(),
            method: "test_method".to_string(),
            success: true,
            response_time: Duration::from_millis(100),
            error_message: None,
        };

        monitoring.record_request(request).await;

        // Check metrics
        let metrics = monitoring.get_metrics().await;
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 0);
    }

    #[tokio::test]
    async fn test_health_check_aggregation() {
        let config = MonitoringConfig::default();
        let monitoring = MonitoringSystem::new(config);

        // Register multiple health checks with different statuses
        let healthy_check = Box::new(BasicHealthCheck::new("healthy".to_string()));
        let degraded_check = Box::new(FileSystemHealthCheck::new(std::path::PathBuf::from(
            "/nonexistent",
        )));

        monitoring.register_health_check(healthy_check).await;
        monitoring.register_health_check(degraded_check).await;

        // Run health check
        let report = monitoring.health_check().await;

        // Overall status should be the worst individual status
        assert_eq!(report.checks.len(), 2);
        // The overall status will be unhealthy due to the nonexistent path
        assert_eq!(report.status, HealthStatus::Unhealthy);
    }
}
