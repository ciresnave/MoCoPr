//! Performance Analysis Example
//!
//! This example demonstrates how to use the monitoring system to collect
//! performance data and optimize MoCoPr server performance.

use mocopr_core::monitoring::{
    BasicHealthCheck, MonitoringConfig, MonitoringSystem, RequestMetrics,
};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting performance analysis example");

    // Create monitoring system with config
    let config = MonitoringConfig::default();
    let monitoring = MonitoringSystem::new(config);

    // Register basic health check
    let health_check = Box::new(BasicHealthCheck::new("system".to_string()));
    monitoring.register_health_check(health_check).await;

    // Start periodic health checks
    monitoring.start_periodic_health_checks().await;

    // Run performance analysis
    run_performance_analysis(&monitoring).await?;

    // Generate performance report
    generate_performance_report(&monitoring).await?;

    Ok(())
}

async fn run_performance_analysis(monitoring: &MonitoringSystem) -> anyhow::Result<()> {
    info!("Running performance analysis scenarios");

    // Scenario 1: Baseline performance
    info!("Scenario 1: Baseline performance measurement");
    let baseline_start = Instant::now();

    for i in 0..100 {
        let start = Instant::now();

        // Simulate tool call
        simulate_tool_call(i).await;

        let duration = start.elapsed();
        let request = RequestMetrics {
            start_time: start,
            method: format!("tool_call_{}", i),
            success: true,
            response_time: duration,
            error_message: None,
        };
        monitoring.record_request(request).await;

        if i % 10 == 0 {
            info!("Processed {} requests", i);
        }
    }

    let baseline_total = baseline_start.elapsed();
    info!("Baseline scenario completed in {:?}", baseline_total);

    // Scenario 2: High-load performance
    info!("Scenario 2: High-load performance measurement");
    let high_load_start = Instant::now();

    let mut tasks = Vec::new();
    for i in 0..50 {
        let task = tokio::spawn(async move {
            let start = Instant::now();
            simulate_tool_call(i).await;
            let duration = start.elapsed();

            (start, format!("concurrent_tool_call_{}", i), duration)
        });
        tasks.push(task);
    }

    for task in tasks {
        let (start, method, duration) = task.await?;
        let request = RequestMetrics {
            start_time: start,
            method,
            success: true,
            response_time: duration,
            error_message: None,
        };
        monitoring.record_request(request).await;
    }

    let high_load_total = high_load_start.elapsed();
    info!("High-load scenario completed in {:?}", high_load_total);

    // Scenario 3: Memory-intensive operations
    info!("Scenario 3: Memory-intensive operations");
    let memory_start = Instant::now();

    for i in 0..20 {
        let start = Instant::now();
        simulate_memory_intensive_operation(i).await;
        let duration = start.elapsed();

        let request = RequestMetrics {
            start_time: start,
            method: format!("memory_operation_{}", i),
            success: true,
            response_time: duration,
            error_message: None,
        };
        monitoring.record_request(request).await;

        // Check health
        let _health_report = monitoring.health_check().await;
    }

    let memory_total = memory_start.elapsed();
    info!("Memory-intensive scenario completed in {:?}", memory_total);

    Ok(())
}

async fn simulate_tool_call(request_id: usize) {
    // Simulate variable processing time
    let processing_time = match request_id % 5 {
        0 => Duration::from_millis(50),  // Fast operation
        1 => Duration::from_millis(100), // Medium operation
        2 => Duration::from_millis(200), // Slow operation
        3 => Duration::from_millis(75),  // Variable operation
        _ => Duration::from_millis(125), // Default operation
    };

    sleep(processing_time).await;

    // Simulate occasional errors
    if request_id.is_multiple_of(23) {
        // This would be an error in real usage
        warn!("Simulated error for request {}", request_id);
    }
}

async fn simulate_memory_intensive_operation(request_id: usize) {
    // Simulate memory allocation
    let _large_data: Vec<u8> = vec![0; 1024 * 1024]; // 1MB allocation

    // Simulate processing time
    sleep(Duration::from_millis(300)).await;

    info!("Memory-intensive operation {} completed", request_id);
}

async fn generate_performance_report(monitoring: &MonitoringSystem) -> anyhow::Result<()> {
    info!("Generating performance report");

    let health_report = monitoring.health_check().await;
    let metrics = monitoring.get_metrics().await;

    println!("\n=== MoCoPr Performance Analysis Report ===");
    println!("Health Status: {:?}", health_report.status);
    println!(
        "Health Checks: {} checks completed",
        health_report.checks.len()
    );

    println!("\nPerformance Metrics:");
    println!("- Total Requests: {}", metrics.total_requests);
    println!("- Successful Requests: {}", metrics.successful_requests);
    println!("- Failed Requests: {}", metrics.failed_requests);
    println!(
        "- Average Response Time: {:.2}ms",
        metrics.avg_response_time_ms
    );
    println!("- P95 Response Time: {:.2}ms", metrics.p95_response_time_ms);
    println!("- P99 Response Time: {:.2}ms", metrics.p99_response_time_ms);
    println!("- Active Connections: {}", metrics.active_connections);
    println!("- Memory Usage: {} bytes", metrics.memory_usage_bytes);
    println!("- CPU Usage: {:.1}%", metrics.cpu_usage_percent);

    // Performance optimization recommendations
    println!("\n=== Performance Optimization Recommendations ===");

    if metrics.avg_response_time_ms > 100.0 {
        println!("⚠️  High average response time detected:");
        println!("   - Consider implementing response caching");
        println!("   - Optimize database queries");
        println!("   - Review algorithm complexity");
    }

    if metrics.p99_response_time_ms > 500.0 {
        println!("⚠️  High P99 response time detected:");
        println!("   - Investigate tail latency causes");
        println!("   - Consider request timeouts");
        println!("   - Review resource contention");
    }

    if metrics.failed_requests > 0 {
        println!("⚠️  Failed requests detected:");
        println!("   - Review error handling");
        println!("   - Implement circuit breakers");
        println!("   - Add retry logic");
    }

    let success_rate = if metrics.total_requests > 0 {
        (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0
    } else {
        0.0
    };

    if success_rate < 99.0 {
        println!("⚠️  Low success rate ({:.1}%):", success_rate);
        println!("   - Implement better error recovery");
        println!("   - Add input validation");
        println!("   - Review system dependencies");
    }

    // Memory optimization recommendations
    println!("\n=== Memory Optimization Recommendations ===");
    if metrics.memory_usage_bytes > 100 * 1024 * 1024 {
        // 100MB threshold
        println!("⚠️  High memory usage detected:");
        println!("   - Implement memory pooling");
        println!("   - Review data structure choices");
        println!("   - Consider streaming for large data");
    }

    // Concurrency optimization recommendations
    println!("\n=== Concurrency Optimization Recommendations ===");
    let requests_per_second = if metrics.avg_response_time_ms > 0.0 {
        1000.0 / metrics.avg_response_time_ms
    } else {
        0.0
    };

    if requests_per_second < 100.0 {
        println!("⚠️  Low throughput detected:");
        println!("   - Increase async task concurrency");
        println!("   - Review blocking operations");
        println!("   - Consider connection pooling");
    }

    // Configuration recommendations
    println!("\n=== Configuration Recommendations ===");
    println!("✅ Recommended server configuration:");
    println!("   - Worker threads: {}", num_cpus::get());
    println!("   - Connection pool size: {}", num_cpus::get() * 2);
    println!("   - Request timeout: 30s");
    println!("   - Maximum concurrent requests: 1000");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_analysis() {
        let config = MonitoringConfig::default();
        let monitoring = MonitoringSystem::new(config);

        // Register health check
        let health_check = Box::new(BasicHealthCheck::new("test".to_string()));
        monitoring.register_health_check(health_check).await;

        // Run a small performance test
        for i in 0..10 {
            let start = Instant::now();
            simulate_tool_call(i).await;
            let duration = start.elapsed();

            let request = RequestMetrics {
                start_time: start,
                method: format!("test_call_{}", i),
                success: true,
                response_time: duration,
                error_message: None,
            };
            monitoring.record_request(request).await;
        }

        let metrics = monitoring.get_metrics().await;
        assert!(metrics.total_requests >= 10);
        assert!(metrics.avg_response_time_ms > 0.0);
    }

    #[tokio::test]
    async fn test_memory_intensive_monitoring() {
        let config = MonitoringConfig::default();
        let monitoring = MonitoringSystem::new(config);

        // Register health check
        let health_check = Box::new(BasicHealthCheck::new("memory_test".to_string()));
        monitoring.register_health_check(health_check).await;

        // Run memory-intensive test
        for i in 0..5 {
            let start = Instant::now();
            simulate_memory_intensive_operation(i).await;
            let duration = start.elapsed();

            let request = RequestMetrics {
                start_time: start,
                method: format!("memory_test_{}", i),
                success: true,
                response_time: duration,
                error_message: None,
            };
            monitoring.record_request(request).await;
        }

        let health_report = monitoring.health_check().await;
        assert!(!health_report.checks.is_empty());

        let metrics = monitoring.get_metrics().await;
        assert!(metrics.total_requests >= 5);
    }
}
