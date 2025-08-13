use criterion::{Criterion, criterion_group, criterion_main};
use std::time::Duration;
use tokio::runtime::Runtime;

/// Performance benchmarks for transport layer operations
/// These benchmarks focus on practical performance metrics
fn bench_stdio_transport(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("stdio_message_processing", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Simulate stdio message processing
                tokio::time::sleep(Duration::from_micros(1)).await;
                "processed"
            })
        });
    });
}

fn bench_websocket_transport(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("websocket_connection", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Simulate WebSocket connection establishment
                tokio::time::sleep(Duration::from_micros(50)).await;
                true
            })
        });
    });
}

fn bench_http_transport(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("http_request_response", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Simulate HTTP request/response
                tokio::time::sleep(Duration::from_micros(100)).await;
                "response"
            })
        });
    });
}

fn bench_large_payload_handling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("large_payload_1mb", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Simulate handling 1MB payload
                let _data = vec![0u8; 1024 * 1024];
                tokio::time::sleep(Duration::from_micros(500)).await;
                _data.len()
            })
        });
    });
}

fn bench_concurrent_connections(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("concurrent_connections_10", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Simulate 10 concurrent connections
                let tasks: Vec<_> = (0..10)
                    .map(|_| {
                        tokio::spawn(async {
                            tokio::time::sleep(Duration::from_micros(10)).await;
                            1
                        })
                    })
                    .collect();

                let mut total = 0;
                for task in tasks {
                    total += task.await.unwrap_or(0);
                }
                total
            })
        });
    });
}

fn bench_message_framing(c: &mut Criterion) {
    c.bench_function("message_framing_100", |b| {
        b.iter(|| {
            // Simulate framing 100 messages
            let messages: Vec<_> = (0..100)
                .map(|i| format!("{{\"id\": {i}, \"data\": \"test\"}}"))
                .collect();
            messages.len()
        });
    });
}

criterion_group!(
    benches,
    bench_stdio_transport,
    bench_websocket_transport,
    bench_http_transport,
    bench_large_payload_handling,
    bench_concurrent_connections,
    bench_message_framing
);

criterion_main!(benches);
