/// Performance benchmarks for transport layer operations
/// These benchmarks measure throughput, latency, and resource usage
/// across different transport mechanisms and message patterns.
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use serde_json::json;
use std::time::Duration;
use tokio::runtime::Runtime;

// Helper function to create test messages
async fn create_test_messages(count: usize) -> Vec<String> {
    (0..count)
        .map(|i| {
            json!({
                "jsonrpc": "2.0",
                "id": i,
                "method": "test/message",
                "params": {"data": format!("message_{}", i)}
            })
            .to_string()
        })
        .collect()
}

// Helper function to simulate message processing
async fn process_stdio_messages(messages: Vec<String>) {
    for message in messages {
        // Simulate processing time
        let _parsed: serde_json::Value = serde_json::from_str(&message).unwrap();
        tokio::task::yield_now().await;
    }
}

fn benchmark_stdio_transport(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("stdio_transport");

    // Benchmark different message sizes
    for size in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(
            BenchmarkId::new("message_processing", size),
            size,
            |b, &size| {
                b.iter(|| {
                    rt.block_on(async {
                        let messages = create_test_messages(size).await;
                        process_stdio_messages(messages).await
                    })
                });
            },
        );
    }

    group.finish();
}

fn bench_websocket_transport(c: &mut Criterion) {
    let mut group = c.benchmark_group("websocket_transport");
    let rt = Runtime::new().unwrap();

    group.bench_function("connection_setup", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Simulate WebSocket connection setup
                tokio::time::sleep(Duration::from_millis(1)).await;
            })
        });
    });

    group.bench_function("message_send", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Simulate WebSocket message send
                tokio::time::sleep(Duration::from_micros(10)).await;
            })
        });
    });

    group.finish();
}

fn bench_http_transport(c: &mut Criterion) {
    let mut group = c.benchmark_group("http_transport");
    let rt = Runtime::new().unwrap();

    group.bench_function("request_response", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Simulate HTTP request/response
                tokio::time::sleep(Duration::from_millis(5)).await;
            })
        });
    });

    group.finish();
}

fn bench_transport_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("transport_throughput");
    let rt = Runtime::new().unwrap();

    for message_count in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*message_count as u64));

        group.bench_with_input(
            BenchmarkId::new("bulk_messages", message_count),
            message_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async move {
                        for _ in 0..count {
                            // Simulate message processing
                            tokio::time::sleep(Duration::from_nanos(100)).await;
                        }
                    })
                });
            },
        );
    }

    group.finish();
}

fn bench_message_serialization_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_serialization_overhead");

    let message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "test/method",
        "params": {"data": "test_data".repeat(100)}
    });

    group.bench_function("serialize", |b| {
        b.iter(|| serde_json::to_string(&message).unwrap());
    });

    let serialized = serde_json::to_string(&message).unwrap();
    group.bench_function("deserialize", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(&serialized).unwrap());
    });

    group.finish();
}

fn bench_concurrent_connections(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_connections");
    let rt = Runtime::new().unwrap();

    for connection_count in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_processing", connection_count),
            connection_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async move {
                        let tasks: Vec<_> = (0..count)
                            .map(|_| {
                                tokio::spawn(async {
                                    // Simulate connection processing
                                    tokio::time::sleep(Duration::from_millis(1)).await;
                                })
                            })
                            .collect();

                        for task in tasks {
                            task.await.unwrap();
                        }
                    })
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    transport_benches,
    benchmark_stdio_transport,
    bench_websocket_transport,
    bench_http_transport,
    bench_transport_throughput,
    bench_message_serialization_overhead,
    bench_concurrent_connections
);
criterion_main!(transport_benches);
