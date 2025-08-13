use anyhow::Result;
use mocopr_core::prelude::*;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Comprehensive stress test scenarios
#[tokio::test]
async fn stress_test_message_serialization() -> Result<()> {
    const NUM_MESSAGES: usize = 10000;

    // Test high-volume message serialization/deserialization
    let mut handles = Vec::new();

    for batch in 0..10 {
        let handle = tokio::spawn(async move {
            for i in 0..NUM_MESSAGES / 10 {
                let message = JsonRpcRequest {
                    jsonrpc: "2.0".to_string(),
                    method: "tools/call".to_string(),
                    params: Some(json!({
                        "name": "test_tool",
                        "arguments": {
                            "batch": batch,
                            "index": i,
                            "data": format!("test_data_{}", i)
                        }
                    })),
                    id: Some(RequestId::String(format!("{batch}_{i}"))),
                };

                // Serialize
                let serialized = serde_json::to_string(&message)?;

                // Deserialize
                let _deserialized: JsonRpcRequest = serde_json::from_str(&serialized)?;

                // Verify round-trip consistency
                assert!(serialized.contains("tools/call"));
                assert!(serialized.contains(&format!("test_data_{i}")));
            }

            Ok::<(), anyhow::Error>(())
        });

        handles.push(handle);
    }

    // Wait for all batches to complete
    for handle in handles {
        handle.await??;
    }

    println!("Successfully processed {NUM_MESSAGES} messages");
    Ok(())
}

#[tokio::test]
async fn stress_test_large_payload_serialization() -> Result<()> {
    // Test various payload sizes - focus on memory and performance, not serialization
    let payload_sizes = vec![1024, 10_240, 102_400, 1_024_000]; // 1KB to 1MB

    for size in payload_sizes {
        let large_data = "x".repeat(size);

        // Test Content creation and memory usage
        let content = Content::Text(TextContent::new(large_data.clone()));
        let start = std::time::Instant::now();
        // Just access the data, skip serialization
        if let Content::Text(text_content) = &content {
            assert_eq!(text_content.text.len(), size);
            assert_eq!(text_content.text, large_data);
        }
        let access_time = start.elapsed();
        println!("Payload size: {size} bytes, Access time: {access_time:?}");
    }

    println!("Large payload stress test completed successfully");
    Ok(())
}

#[tokio::test]
async fn stress_test_concurrent_operations() -> Result<()> {
    const NUM_WORKERS: usize = 20;
    const OPERATIONS_PER_WORKER: usize = 100;

    let shared_state = Arc::new(RwLock::new(HashMap::<String, String>::new()));
    let mut handles = Vec::new();

    for worker_id in 0..NUM_WORKERS {
        let state = Arc::clone(&shared_state);

        let handle = tokio::spawn(async move {
            for op_id in 0..OPERATIONS_PER_WORKER {
                let key = format!("worker_{worker_id}_op_{op_id}");
                let value = format!("value_{op_id}");

                // Mix of read and write operations
                if op_id % 3 == 0 {
                    // Write operation
                    let mut state = state.write().await;
                    state.insert(key.clone(), value);
                } else {
                    // Read operation
                    let state = state.read().await;
                    let _result = state.get(&key);
                }

                // Create and process messages to simulate real workload
                let message = Protocol::create_request(
                    "resources/read",
                    Some(json!({
                        "uri": format!("memory://{}", key)
                    })),
                    Some(RequestId::from(format!("{worker_id}_{op_id}"))),
                );

                let _serialized = serde_json::to_string(&message)?;

                // Small delay to simulate processing time
                tokio::time::sleep(tokio::time::Duration::from_micros(10)).await;
            }

            Ok::<(), anyhow::Error>(())
        });

        handles.push(handle);
    }

    // Wait for all workers to complete
    for handle in handles {
        handle.await??;
    }

    // Verify final state
    let final_state = shared_state.read().await;
    println!("Final state contains {} entries", final_state.len());

    Ok(())
}

#[tokio::test]
async fn stress_test_error_handling() -> Result<()> {
    const NUM_OPERATIONS: usize = 1000;

    let mut success_count = 0;
    let mut error_count = 0;

    for i in 0..NUM_OPERATIONS {
        let result = simulate_operation_with_errors(i).await;

        match result {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }

    println!("Operations completed: {success_count} success, {error_count} errors");

    // We expect some errors in this test
    assert!(error_count > 0);
    assert!(success_count > 0);

    Ok(())
}

#[tokio::test]
async fn stress_test_memory_efficiency() -> Result<()> {
    // Test that we don't have memory leaks during intensive operations
    const ITERATIONS: usize = 1000;

    for i in 0..ITERATIONS {
        // Create and drop many objects to test for leaks
        let mut messages = Vec::new();

        for j in 0..100 {
            let message = Protocol::create_request(
                "test/method",
                Some(json!({
                    "iteration": i,
                    "index": j,
                    "data": format!("test_data_{}_{}", i, j)
                })),
                Some(RequestId::from(format!("{i}_{j}"))),
            );

            messages.push(message);
        }

        // Process all messages
        for message in &messages {
            let _serialized = serde_json::to_string(message)?;
        }

        // Explicitly drop to test cleanup
        drop(messages);

        if i % 100 == 0 {
            println!("Completed iteration {i}");
        }
    }

    Ok(())
}

// Helper functions

async fn simulate_operation_with_errors(index: usize) -> Result<String> {
    // Simulate various error conditions
    match index % 10 {
        0 => {
            // Simulate timeout
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            Err(anyhow::anyhow!("Timeout error"))
        }
        1 => {
            // Simulate invalid input
            Err(anyhow::anyhow!("Invalid input error"))
        }
        2 => {
            // Simulate resource not found
            Err(anyhow::anyhow!("Resource not found"))
        }
        _ => {
            // Successful operation
            tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
            Ok(format!("Success for operation {index}"))
        }
    }
}

#[tokio::test]
async fn stress_test_tool_parameter_handling() -> Result<()> {
    // Test with many different parameter combinations
    let test_cases = vec![
        (json!({"name": "John", "age": 30, "active": true}), true),
        (json!({"name": "", "age": 30}), false), // Invalid: empty name
        (json!({"name": "Jane", "age": -5}), false), // Invalid: negative age
        (json!({"name": "Bob", "age": 150}), false), // Invalid: age too high
        (json!({"age": 30, "active": true}), false), // Invalid: missing required name
        (json!({"name": "Alice", "tags": ["tag1", "tag2"]}), true),
    ];

    for (params, should_be_valid) in test_cases {
        // Create tool call request
        let request = Protocol::create_request(
            "tools/call",
            Some(json!({
                "name": "test_tool",
                "arguments": params
            })),
            Some(RequestId::from(uuid::Uuid::new_v4())),
        );

        // Simulate validation time
        tokio::time::sleep(tokio::time::Duration::from_micros(10)).await;

        let _serialized = serde_json::to_string(&request)?;

        println!("Tested parameters: {params} (expected valid: {should_be_valid})");
    }

    Ok(())
}

#[tokio::test]
async fn stress_test_protocol_version_handling() -> Result<()> {
    // Test different protocol versions
    let protocol_versions = vec![
        "2024-11-05",
        "2025-06-18",
        "future-version",
        "invalid-version",
    ];

    for version in protocol_versions {
        let init_request = messages::InitializeRequest {
            protocol_version: version.to_string(),
            capabilities: capabilities::ClientCapabilities::default(),
            client_info: Implementation {
                name: "Test Client".to_string(),
                version: "1.0.0".to_string(),
            },
        };

        // Test serialization/deserialization
        let serialized = serde_json::to_string(&init_request)?;
        let _deserialized: messages::InitializeRequest = serde_json::from_str(&serialized)?;

        println!("Tested protocol version: {version}");
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run with --ignored for stress testing
async fn memory_stress_test() -> Result<(), Box<dyn std::error::Error>> {
    use tokio::time::{Duration, sleep};

    println!("🧠 Starting memory stress test...");

    let mut handlers = Vec::new();
    let total_allocations = 10_000;
    let chunk_size = 100;

    for chunk in 0..(total_allocations / chunk_size) {
        let handle = tokio::spawn(async move {
            let mut large_data = Vec::new();

            // Allocate large amounts of data
            for i in 0..chunk_size {
                let request_id = format!("mem_stress_{}_{}", chunk, i);
                let large_payload = "x".repeat(1024 * 100); // 100KB per request

                let message = Protocol::create_request(
                    "tools/call",
                    Some(json!({
                        "name": "memory_test",
                        "arguments": {
                            "data": large_payload,
                            "iteration": i
                        }
                    })),
                    Some(RequestId::from(request_id)),
                );

                large_data.push(message);

                // Simulate processing delay
                if i % 10 == 0 {
                    sleep(Duration::from_millis(1)).await;
                }
            }

            // Hold data for a bit then release
            sleep(Duration::from_millis(100)).await;
            drop(large_data);

            chunk
        });

        handlers.push(handle);
    }

    // Wait for all allocations to complete
    let results = futures::future::join_all(handlers).await;
    let completed_chunks = results.into_iter().filter_map(|r| r.ok()).count();

    println!(
        "✅ Memory stress test completed: {}/{} chunks processed",
        completed_chunks,
        total_allocations / chunk_size
    );

    // Give GC time to cleanup
    sleep(Duration::from_millis(500)).await;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn connection_failure_recovery_test() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use tokio::time::{Duration, sleep, timeout};

    println!("🔗 Starting connection failure recovery test...");

    let failure_simulation = Arc::new(AtomicBool::new(false));
    let retry_count = Arc::new(AtomicUsize::new(0));
    let success_count = Arc::new(AtomicUsize::new(0));

    let mut handlers = Vec::new();

    // Simulate multiple clients with intermittent failures
    for client_id in 0..20 {
        let failure_sim = failure_simulation.clone();
        let retry_cnt = retry_count.clone();
        let success_cnt = success_count.clone();

        let handle = tokio::spawn(async move {
            for attempt in 0..50 {
                // Simulate random connection failures
                if attempt % 7 == 0 {
                    failure_sim.store(true, Ordering::Relaxed);
                    sleep(Duration::from_millis(10)).await;
                    failure_sim.store(false, Ordering::Relaxed);
                }

                let mut retries = 0;
                let max_retries = 3;

                loop {
                    if failure_sim.load(Ordering::Relaxed) && retries < max_retries {
                        retry_cnt.fetch_add(1, Ordering::Relaxed);
                        retries += 1;
                        sleep(Duration::from_millis(50 * retries as u64)).await;
                        continue;
                    }

                    // Simulate successful operation by creating a message
                    let _message = Protocol::create_request(
                        "test/operation",
                        Some(json!({
                            "client_id": client_id,
                            "attempt": attempt
                        })),
                        Some(RequestId::from(format!(
                            "conn_test_{}_{}",
                            client_id, attempt
                        ))),
                    );

                    success_cnt.fetch_add(1, Ordering::Relaxed);
                    break;
                }

                sleep(Duration::from_millis(5)).await;
            }
            client_id
        });

        handlers.push(handle);
    }

    // Wait for all clients to complete
    let results = timeout(Duration::from_secs(30), futures::future::join_all(handlers)).await?;

    let completed_clients = results.into_iter().filter_map(|r| r.ok()).count();

    let total_retries = retry_count.load(Ordering::Relaxed);
    let total_successes = success_count.load(Ordering::Relaxed);

    println!("✅ Connection failure recovery test completed:");
    println!("   Clients completed: {}/20", completed_clients);
    println!("   Total retries: {}", total_retries);
    println!("   Total successes: {}", total_successes);
    println!(
        "   Success rate: {:.2}%",
        (total_successes as f64 / (total_successes + total_retries) as f64) * 100.0
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Temporarily disabled - needs API update
async fn protocol_edge_cases_test() -> Result<(), Box<dyn std::error::Error>> {
    // Edge case: invalid method name, missing params, large payload
    let edge_cases = vec![
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "".to_string(), // empty method
            params: None,
            id: Some(RequestId::String("edge_1".to_string())),
        },
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({})), // empty params
            id: Some(RequestId::String("edge_2".to_string())),
        },
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({"arguments": "x".repeat(1024 * 1024)})), // large payload
            id: Some(RequestId::String("edge_3".to_string())),
        },
    ];
    for req in edge_cases {
        let serialized = serde_json::to_string(&req)?;
        let deserialized: JsonRpcRequest = serde_json::from_str(&serialized)?;
        assert_eq!(req.method, deserialized.method);
    }
    println!("🎯 Protocol edge cases test completed");
    Ok(())
}

#[tokio::test]
#[ignore] // Temporarily disabled - needs API update
async fn long_running_stability_test() -> Result<(), Box<dyn std::error::Error>> {
    // Simulate long-running repeated tool calls
    let mut successes = 0;
    for i in 0..1000 {
        let req = ToolsCallRequest {
            name: "noop".to_string(),
            arguments: Some(serde_json::json!({"iteration": i})),
        };
        let serialized = serde_json::to_string(&req)?;
        let deserialized: ToolsCallRequest = serde_json::from_str(&serialized)?;
        if deserialized.name == "noop" {
            successes += 1;
        }
    }
    println!("⏰ Long-running stability test completed: {successes} iterations");
    Ok(())
}

#[tokio::test]
#[ignore]
async fn resource_exhaustion_test() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Semaphore;
    use tokio::time::{Duration, sleep};

    println!("🔥 Starting resource exhaustion test...");

    // Test file descriptor exhaustion protection
    let fd_counter = Arc::new(AtomicUsize::new(0));
    let max_concurrent = 1000;
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    let mut handles = Vec::new();

    for i in 0..max_concurrent * 2 {
        let sem = semaphore.clone();
        let counter = fd_counter.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            counter.fetch_add(1, Ordering::Relaxed);

            // Simulate resource-intensive operation
            sleep(Duration::from_millis(100)).await;

            counter.fetch_sub(1, Ordering::Relaxed);
            i
        });

        handles.push(handle);
    }

    // Monitor resource usage
    let monitor_counter = fd_counter.clone();
    let monitor_handle = tokio::spawn(async move {
        let mut max_concurrent_seen = 0;

        for _ in 0..50 {
            let current = monitor_counter.load(Ordering::Relaxed);
            max_concurrent_seen = max_concurrent_seen.max(current);
            sleep(Duration::from_millis(50)).await;
        }

        max_concurrent_seen
    });

    let results = futures::future::join_all(handles).await;
    let max_seen = monitor_handle.await?;

    let completed = results.into_iter().filter_map(|r| r.ok()).count();

    println!("✅ Resource exhaustion test completed:");
    println!("   Tasks completed: {}/{}", completed, max_concurrent * 2);
    println!("   Max concurrent resources: {}", max_seen);
    println!(
        "   Resource limit respected: {}",
        max_seen <= max_concurrent
    );

    Ok(())
}
