use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mocopr_core::{JsonRpcRequest, RequestId, Tool, ToolParameter, utils::json};
use serde_json::json;
use std::time::Duration;

fn bench_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_serialization");

    // Benchmark different message sizes
    for size in [1, 10, 100, 1000].iter() {
        let message = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(RequestId::Number(1)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "test_tool",
                "arguments": {
                    "data": "x".repeat(*size)
                }
            })),
        };

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::new("serialize", size), size, |b, _size| {
            b.iter(|| json::to_string(&message).unwrap());
        });

        let serialized = json::to_string(&message).unwrap();
        group.bench_with_input(BenchmarkId::new("deserialize", size), size, |b, _size| {
            b.iter(|| json::from_str::<JsonRpcRequest>(&serialized).unwrap());
        });
    }
    group.finish();
}

fn bench_tool_parameter_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_parameter_validation");

    let _tool_param = ToolParameter::new("string".to_string())
        .with_description("A test parameter")
        .required();

    let test_values = vec![
        json!("valid_string"),
        json!(123),
        json!(null),
        json!({"invalid": "object"}),
    ];

    group.bench_function("parameter_validation", |b| {
        b.iter(|| {
            for value in &test_values {
                // Simple validation simulation
                let _is_valid = value.is_string();
            }
        });
    });

    group.finish();
}

fn bench_tool_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_creation");

    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "number"}
        },
        "required": ["name"]
    });

    group.bench_function("tool_new", |b| {
        b.iter(|| {
            Tool::new("test_tool", schema.clone()).with_description("A test tool for benchmarking");
        });
    });

    group.finish();
}

fn bench_resource_uri_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("resource_uri_parsing");

    let uris = vec![
        "file:///path/to/file.txt",
        "https://example.com/api/data",
        "memory://cache/key123",
        "custom://namespace/resource?param=value",
    ];

    group.bench_function("uri_parsing", |b| {
        b.iter(|| {
            for uri in &uris {
                // Simulate URI parsing
                let _parsed = uri.starts_with("file://")
                    || uri.starts_with("https://")
                    || uri.starts_with("memory://")
                    || uri.starts_with("custom://");
            }
        });
    });

    group.finish();
}

fn bench_resource_content_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("resource_content_serialization");

    // Benchmark different content sizes
    for size_kb in [1, 10, 100, 1000].iter() {
        let content_size = size_kb * 1024;
        let text_content = "A".repeat(content_size);

        group.throughput(Throughput::Bytes(content_size as u64));
        group.bench_with_input(
            BenchmarkId::new("text_serialize", size_kb),
            &text_content,
            |b, content| {
                b.iter(|| json::to_string(content).unwrap());
            },
        );

        let serialized = json::to_string(&text_content).unwrap();
        group.bench_with_input(
            BenchmarkId::new("text_deserialize", size_kb),
            &serialized,
            |b, data| {
                b.iter(|| json::from_str::<String>(data).unwrap());
            },
        );
    }
    group.finish();
}

fn bench_large_payload_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_payload_processing");

    // Create a batch of 100 requests
    let messages: Vec<JsonRpcRequest> = (0..100)
        .map(|i| JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(RequestId::Number(i)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "batch_tool",
                "arguments": {"index": i, "data": format!("payload_{}", i)}
            })),
        })
        .collect();

    group.bench_function("batch_serialize", |b| {
        b.iter(|| {
            for message in &messages {
                let _serialized = json::to_string(message).unwrap();
            }
        });
    });

    let serialized_messages: Vec<String> = messages
        .iter()
        .map(|msg| json::to_string(msg).unwrap())
        .collect();

    group.bench_function("batch_deserialize", |b| {
        b.iter(|| {
            for serialized in &serialized_messages {
                let _deserialized: JsonRpcRequest = json::from_str(serialized).unwrap();
            }
        });
    });

    group.finish();
}

fn bench_concurrent_message_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_message_processing");
    group.measurement_time(Duration::from_secs(5));

    let message = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(RequestId::Number(1)),
        method: "test/method".to_string(),
        params: Some(json!({"test": "data"})),
    };

    let serialized = json::to_string(&message).unwrap();

    group.bench_function("message_roundtrip", |b| {
        b.iter(|| {
            let _serialized = json::to_string(&message).unwrap();
            let _deserialized: JsonRpcRequest = json::from_str(&serialized).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_message_serialization,
    bench_tool_parameter_validation,
    bench_tool_creation,
    bench_resource_uri_parsing,
    bench_resource_content_serialization,
    bench_large_payload_processing,
    bench_concurrent_message_processing
);

criterion_main!(benches);
