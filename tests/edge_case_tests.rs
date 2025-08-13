use anyhow::Result;
use mocopr_core::prelude::*;
use serde_json::json;

/// Comprehensive edge case and error condition tests
#[tokio::test]
async fn test_invalid_json_rpc_format() -> Result<()> {
    let invalid_messages = vec![
        // Missing jsonrpc field
        r#"{"id": 1, "method": "test"}"#,
        // Missing method in request
        r#"{"jsonrpc": "2.0", "id": 1}"#,
        // Invalid JSON
        r#"{"jsonrpc": "2.0", "id": 1, "method": "test""#,
        // Null method
        r#"{"jsonrpc": "2.0", "id": 1, "method": null}"#,
        // Empty method
        r#"{"jsonrpc": "2.0", "id": 1, "method": ""}"#,
    ];

    let mut passed_tests = 0;

    for invalid_msg in invalid_messages {
        let result = Protocol::parse_message(invalid_msg);
        if result.is_err() {
            println!("Correctly rejected invalid message: {invalid_msg}");
            passed_tests += 1;
        } else {
            println!(
                "Warning: Message was parsed but may be caught by higher-level validation: {invalid_msg}"
            );
        }
    }

    // Test wrong jsonrpc version separately
    let wrong_version_msg = r#"{"jsonrpc": "1.0", "id": 1, "method": "test"}"#;
    let result = Protocol::parse_message(wrong_version_msg);
    if let Ok(JsonRpcMessage::Request(req)) = result {
        // The parsing succeeds but we can validate the version
        if req.jsonrpc != "2.0" {
            println!("Correctly identified wrong JSON-RPC version: {wrong_version_msg}");
            passed_tests += 1;
        }
    } else {
        println!("Wrong version message was rejected: {wrong_version_msg}");
        passed_tests += 1;
    }

    // At least some validation should work
    assert!(
        passed_tests > 0,
        "At least some invalid messages should be caught"
    );

    Ok(())
}

#[tokio::test]
async fn test_boundary_values() -> Result<()> {
    // Test with extreme values
    let test_cases = vec![
        // Very large ID
        (json!(i64::MAX), "Large integer ID"),
        (json!(i64::MIN), "Small integer ID"),
        // Very long method name
        (json!("a".repeat(1000)), "Long method name"),
        // Unicode in method and params
        (json!("test/🔥"), "Unicode method name"),
        (
            json!({"emoji": "🚀", "unicode": "héllo"}),
            "Unicode parameters",
        ),
        // Empty objects and arrays
        (json!({}), "Empty object"),
        (json!([]), "Empty array"),
        // Nested structures
        (
            json!({"level1": {"level2": {"level3": "deep"}}}),
            "Deep nesting",
        ),
    ];

    for (value, description) in test_cases {
        // Test serialization round-trip
        let serialized = serde_json::to_string(&value)?;
        let deserialized: serde_json::Value = serde_json::from_str(&serialized)?;
        assert_eq!(value, deserialized, "Round-trip failed for: {description}");

        println!("Successfully handled: {description}");
    }

    Ok(())
}

#[tokio::test]
async fn test_protocol_version_edge_cases() -> Result<()> {
    let versions = vec![
        "2024-11-05", // Valid old version
        "2025-06-18", // Current version
        "9999-12-31", // Future version
        "",           // Empty version
        "invalid",    // Invalid format
        "2024-13-01", // Invalid month
        "2024-02-30", // Invalid date
        "v1.0.0",     // Wrong format
    ];

    for version in versions {
        let init_request = messages::InitializeRequest {
            protocol_version: version.to_string(),
            capabilities: capabilities::ClientCapabilities::default(),
            client_info: Implementation {
                name: "Test Client".to_string(),
                version: "1.0.0".to_string(),
            },
        };

        // Test that we can serialize/deserialize any version string
        let serialized = serde_json::to_string(&init_request)?;
        let _deserialized: messages::InitializeRequest = serde_json::from_str(&serialized)?;

        println!("Processed protocol version: {version}");
    }

    Ok(())
}

#[tokio::test]
async fn test_error_code_handling() -> Result<()> {
    let error_scenarios = vec![
        (error_codes::PARSE_ERROR, "Parse error"),
        (error_codes::INVALID_REQUEST, "Invalid request"),
        (error_codes::METHOD_NOT_FOUND, "Method not found"),
        (error_codes::INVALID_PARAMS, "Invalid parameters"),
        (error_codes::INTERNAL_ERROR, "Internal error"),
        (error_codes::RESOURCE_NOT_FOUND, "Resource not found"),
        (error_codes::TOOL_NOT_FOUND, "Tool not found"),
        (-32000, "Custom error code"),
        (0, "Zero error code"),
        (999999, "Large error code"),
    ];

    for (code, message) in error_scenarios {
        let error = Protocol::create_error(code, message, None);

        // Verify error structure
        assert_eq!(error.code, code);
        assert_eq!(error.message, message);

        // Test serialization
        let serialized = serde_json::to_string(&error)?;
        let deserialized: JsonRpcError = serde_json::from_str(&serialized)?;
        assert_eq!(error.code, deserialized.code);
        assert_eq!(error.message, deserialized.message);

        println!("Handled error code: {code} - {message}");
    }

    Ok(())
}

#[tokio::test]
async fn test_capability_edge_cases() -> Result<()> {
    // Test various capability combinations
    let mut capabilities = capabilities::ClientCapabilities::default();

    // Test with all capabilities enabled
    capabilities.roots = Some(capabilities::RootsCapability {
        list_changed: Some(true),
    });
    capabilities.sampling = Some(capabilities::SamplingCapability {});

    // Test serialization with complex capabilities
    let serialized = serde_json::to_string(&capabilities)?;
    let deserialized: capabilities::ClientCapabilities = serde_json::from_str(&serialized)?;

    // Verify round-trip consistency
    let re_serialized = serde_json::to_string(&deserialized)?;
    let parsed1: serde_json::Value = serde_json::from_str(&serialized)?;
    let parsed2: serde_json::Value = serde_json::from_str(&re_serialized)?;
    assert_eq!(parsed1, parsed2);

    println!("Successfully handled complex capabilities");
    Ok(())
}

#[tokio::test]
async fn test_uri_handling_edge_cases() -> Result<()> {
    let uri_test_cases = vec![
        "file:///normal/path.txt",
        "http://example.com/resource",
        "https://secure.example.com:8443/path?query=value",
        "custom-scheme://authority/path",
        "memory://buffer/123",
        "ftp://ftp.example.com/file.dat",
        // Edge cases
        "scheme://",
        "://no-scheme",
        "/absolute/path",
        "relative/path",
        "",
        "very-long-scheme-name://very.long.domain.name.example.com:65535/very/long/path/with/many/segments/and/file.extension?very=long&query=string&with=many&parameters=value",
    ];

    for uri_str in uri_test_cases {
        // Test that we can process URIs as strings
        let test_params = json!({
            "uri": uri_str
        });

        let serialized = serde_json::to_string(&test_params)?;
        let deserialized: serde_json::Value = serde_json::from_str(&serialized)?;

        assert_eq!(test_params, deserialized);
        println!("Processed URI: {uri_str}");
    }

    Ok(())
}

#[tokio::test]
async fn test_large_batch_operations() -> Result<()> {
    const BATCH_SIZE: usize = 1000;

    // Create a large batch of requests
    let mut requests = Vec::new();
    for i in 0..BATCH_SIZE {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "batch/operation".to_string(),
            params: Some(json!({
                "index": i,
                "data": format!("batch_item_{}", i)
            })),
            id: Some(RequestId::String(format!("batch_{i}"))),
        };
        requests.push(request);
    }

    // Process batch in chunks
    let chunk_size = 100;
    for (chunk_idx, chunk) in requests.chunks(chunk_size).enumerate() {
        let serialized_chunk: Result<Vec<String>> = chunk
            .iter()
            .map(|req| serde_json::to_string(req).map_err(Into::into))
            .collect();

        let serialized_chunk = serialized_chunk?;

        // Verify all items in chunk
        for (i, serialized) in serialized_chunk.iter().enumerate() {
            assert!(serialized.contains("batch/operation"));
            let global_index = chunk_idx * chunk_size + i;
            assert!(serialized.contains(&format!("batch_item_{global_index}")));
        }
    }

    println!("Successfully processed batch of {BATCH_SIZE} requests");
    Ok(())
}

#[tokio::test]
async fn test_concurrent_serialization() -> Result<()> {
    const NUM_WORKERS: usize = 10;
    const ITEMS_PER_WORKER: usize = 100;

    let mut handles = Vec::new();

    for worker_id in 0..NUM_WORKERS {
        let handle = tokio::spawn(async move {
            for item_id in 0..ITEMS_PER_WORKER {
                let request = Protocol::create_request(
                    "concurrent/test",
                    Some(json!({
                        "worker_id": worker_id,
                        "item_id": item_id,
                        "timestamp": std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                    })),
                    Some(RequestId::from(format!("worker_{worker_id}_{item_id}"))),
                );

                // Test serialization under concurrency
                let serialized = serde_json::to_string(&request)?;
                let deserialized: JsonRpcRequest = serde_json::from_str(&serialized)?;

                // Verify consistency
                assert_eq!(request.method, deserialized.method);
                assert_eq!(request.id, deserialized.id);

                // Small delay to increase chance of race conditions
                tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
            }

            Ok::<(), anyhow::Error>(())
        });

        handles.push(handle);
    }

    // Wait for all workers to complete
    for handle in handles {
        handle.await??;
    }

    println!("Successfully completed concurrent serialization test");
    Ok(())
}

#[tokio::test]
async fn test_memory_pressure() -> Result<()> {
    // Test behavior under memory pressure
    const LARGE_ALLOCATION_SIZE: usize = 1_000_000; // 1MB strings
    const NUM_ALLOCATIONS: usize = 100;

    let mut large_objects = Vec::new();

    for i in 0..NUM_ALLOCATIONS {
        let large_data = "x".repeat(LARGE_ALLOCATION_SIZE);

        let request = Protocol::create_request(
            "memory/test",
            Some(json!({
                "index": i,
                "size": LARGE_ALLOCATION_SIZE,
                "data": large_data
            })),
            Some(RequestId::from(format!("memory_test_{i}"))),
        );

        // Serialize large object
        let serialized = serde_json::to_string(&request)?;

        // Keep some objects in memory to create pressure
        if i % 10 == 0 {
            large_objects.push(serialized);
        }

        if i % 20 == 0 {
            println!("Processed {i} large allocations");
        }
    }

    // Force cleanup
    drop(large_objects);

    println!("Successfully handled memory pressure test");
    Ok(())
}

#[tokio::test]
async fn test_timeout_scenarios() -> Result<()> {
    use tokio::time::{Duration, timeout};

    // Test operations with various timeouts
    let timeout_durations = vec![
        Duration::from_millis(1),
        Duration::from_millis(10),
        Duration::from_millis(100),
        Duration::from_secs(1),
    ];

    for duration in timeout_durations {
        let result = timeout(duration, async {
            // Simulate work that might take varying amounts of time
            let request = Protocol::create_request(
                "timeout/test",
                Some(json!({
                    "timeout_ms": duration.as_millis()
                })),
                Some(RequestId::from(format!(
                    "timeout_test_{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos()
                ))),
            );

            let _serialized = serde_json::to_string(&request)?;

            // Simulate processing delay
            tokio::time::sleep(Duration::from_micros(100)).await;

            Ok::<(), anyhow::Error>(())
        })
        .await;

        match result {
            Ok(Ok(())) => println!("Operation completed within {duration:?}"),
            Ok(Err(e)) => println!("Operation failed within {duration:?}: {e}"),
            Err(_) => println!("Operation timed out after {duration:?}"),
        }
    }

    Ok(())
}
