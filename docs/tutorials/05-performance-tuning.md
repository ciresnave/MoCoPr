# Performance Tuning Guide

This guide covers performance optimization techniques for MoCoPr applications, from basic tuning to advanced optimization strategies.

## Table of Contents

1. [Performance Fundamentals](#fundamentals)
2. [Benchmarking and Profiling](#benchmarking)
3. [Memory Optimization](#memory-optimization)
4. [CPU Optimization](#cpu-optimization)
5. [I/O Optimization](#io-optimization)
6. [Transport Layer Optimization](#transport-optimization)
7. [Scaling Strategies](#scaling)
8. [Monitoring and Observability](#monitoring)

## Performance Fundamentals {#fundamentals}

### Understanding MCP Performance Characteristics

MCP server performance is influenced by several factors:

- **Message Serialization/Deserialization**: JSON processing overhead
- **Transport Layer**: Network latency and bandwidth
- **Tool Execution Time**: Business logic performance
- **Concurrency Model**: Async task management
- **Memory Usage**: Allocation patterns and garbage collection

### Setting Performance Goals

Define clear performance targets:

```rust
// Example performance requirements
const REQUIREMENTS: PerformanceTargets = PerformanceTargets {
    max_request_latency: Duration::from_millis(100),
    min_throughput_rps: 1000,
    max_memory_usage_mb: 512,
    max_cpu_usage_percent: 80,
};
```

## Benchmarking and Profiling {#benchmarking}

### Built-in Benchmarks

MoCoPr includes comprehensive benchmarks in the `benches/` directory:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark suite
cargo bench protocol_benchmarks
cargo bench transport_benchmarks
cargo bench performance_benchmarks

# Generate detailed reports
cargo bench --bench performance_benchmarks -- --output-format html
```

### Custom Benchmarks

Create application-specific benchmarks:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use mocopr_server::prelude::*;
use tokio::runtime::Runtime;

fn bench_tool_execution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let tool = MyTool::new();

    c.bench_function("tool_execution", |b| {
        b.to_async(&rt).iter(|| async {
            let result = tool.execute(black_box(Some(json!({
                "input": "test_data"
            })))).await;
            black_box(result)
        })
    });
}

fn bench_server_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_with_input(
        BenchmarkId::new("server_throughput", "concurrent_requests"),
        &100,
        |b, &request_count| {
            b.to_async(&rt).iter(|| async {
                let server = create_test_server().await;
                let tasks: Vec<_> = (0..request_count)
                    .map(|_| {
                        let client = create_test_client(&server);
                        tokio::spawn(async move {
                            client.call_tool("test_tool", json!({})).await
                        })
                    })
                    .collect();

                futures::future::join_all(tasks).await
            })
        },
    );
}

criterion_group!(benches, bench_tool_execution, bench_server_throughput);
criterion_main!(benches);
```

### Profiling with perf

For CPU profiling on Linux:

```bash
# Profile the server under load
perf record --call-graph dwarf target/release/your-server

# Generate flame graph
perf script | ../FlameGraph/stackcollapse-perf.pl | ../FlameGraph/flamegraph.pl > profile.svg
```

### Memory Profiling

Use tools like `valgrind` or `heaptrack`:

```bash
# Memory profiling with valgrind
valgrind --tool=massif --stacks=yes ./target/release/your-server

# Analyze results
ms_print massif.out.xxx
```

## Memory Optimization {#memory-optimization}

### Efficient Data Structures

Choose appropriate data structures for your use case:

```rust
use smallvec::SmallVec;
use ahash::AHashMap;
use bytes::Bytes;

#[derive(Tool)]
pub struct OptimizedTool {
    // Use SmallVec for small collections to avoid heap allocation
    recent_requests: SmallVec<[RequestId; 8]>,

    // Use AHashMap for better performance than HashMap
    cache: AHashMap<String, Bytes>,

    // Pre-allocate buffers to avoid repeated allocations
    buffer: Vec<u8>,
}

impl OptimizedTool {
    pub fn new() -> Self {
        Self {
            recent_requests: SmallVec::new(),
            cache: AHashMap::with_capacity(1000),
            // Pre-allocate 64KB buffer
            buffer: Vec::with_capacity(64 * 1024),
        }
    }
}
```

### Memory Pool Pattern

Implement object pooling for frequently allocated objects:

```rust
use object_pool::Pool;
use std::sync::Arc;

pub struct BufferPool {
    pool: Pool<Vec<u8>>,
}

impl BufferPool {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            pool: Pool::new(32, || Vec::with_capacity(8192)),
        })
    }

    pub fn get_buffer(&self) -> object_pool::Reusable<Vec<u8>> {
        let mut buf = self.pool.try_pull().unwrap_or_else(|| {
            self.pool.attach(Vec::with_capacity(8192))
        });
        buf.clear();
        buf
    }
}

#[derive(Tool)]
pub struct PooledTool {
    buffer_pool: Arc<BufferPool>,
}

#[async_trait::async_trait]
impl ToolExecutor for PooledTool {
    async fn execute(&self, arguments: Option<Value>) -> Result<ToolsCallResponse> {
        // Get a buffer from the pool
        let mut buffer = self.buffer_pool.get_buffer();

        // Use the buffer for processing
        self.process_with_buffer(&mut buffer, arguments).await

        // Buffer is automatically returned to pool when dropped
    }
}
```

### Zero-Copy Optimizations

Minimize data copying where possible:

```rust
use bytes::{Bytes, BytesMut};
use serde_json::from_slice;

pub struct ZeroCopyProcessor {
    input_buffer: BytesMut,
}

impl ZeroCopyProcessor {
    pub fn process_request(&mut self, data: Bytes) -> Result<Value> {
        // Use from_slice to avoid copying for deserialization
        let request: JsonRpcRequest = from_slice(&data)?;

        // Process without copying the underlying data
        self.process_zero_copy(&request, data)
    }

    fn process_zero_copy(&self, request: &JsonRpcRequest, raw_data: Bytes) -> Result<Value> {
        // Work directly with byte slices where possible
        if let Some(params) = &request.params {
            // Extract parameter data without copying
            let param_slice = &raw_data[self.find_params_offset(raw_data)?..];
            return self.process_params_slice(param_slice);
        }

        Ok(Value::Null)
    }
}
```

## CPU Optimization {#cpu-optimization}

### Parallel Processing

Use Rayon for CPU-intensive tasks:

```rust
use rayon::prelude::*;

#[derive(Tool)]
pub struct ParallelProcessingTool;

#[async_trait::async_trait]
impl ToolExecutor for ParallelProcessingTool {
    async fn execute(&self, arguments: Option<Value>) -> Result<ToolsCallResponse> {
        let data: Vec<DataItem> = self.parse_input(arguments)?;

        // Use Rayon for parallel processing
        let results: Result<Vec<_>, _> = data
            .par_iter()
            .map(|item| self.process_item(item))
            .collect();

        let processed_results = results?;

        Ok(ToolsCallResponse::success(vec![Content::Text(
            TextContent::new(&serde_json::to_string(&processed_results)?)
        )]))
    }
}
```

### SIMD Optimizations

Use SIMD for mathematical operations:

```rust
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

pub struct SimdCalculator;

impl SimdCalculator {
    #[cfg(target_arch = "x86_64")]
    pub fn vectorized_add(a: &[f32], b: &[f32]) -> Vec<f32> {
        if is_x86_feature_detected!("avx2") {
            unsafe { self.avx2_add(a, b) }
        } else {
            // Fallback to scalar implementation
            a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()
        }
    }

    #[cfg(target_arch = "x86_64")]
    unsafe fn avx2_add(&self, a: &[f32], b: &[f32]) -> Vec<f32> {
        let mut result = Vec::with_capacity(a.len());

        let chunks_a = a.chunks_exact(8);
        let chunks_b = b.chunks_exact(8);

        for (chunk_a, chunk_b) in chunks_a.zip(chunks_b) {
            let va = _mm256_loadu_ps(chunk_a.as_ptr());
            let vb = _mm256_loadu_ps(chunk_b.as_ptr());
            let vr = _mm256_add_ps(va, vb);

            let mut temp = [0.0f32; 8];
            _mm256_storeu_ps(temp.as_mut_ptr(), vr);
            result.extend_from_slice(&temp);
        }

        result
    }
}
```

### Compilation Optimizations

Optimize your `Cargo.toml` for performance:

```toml
[profile.release]
# Enable link-time optimization
lto = true
# Enable more aggressive optimizations
codegen-units = 1
# Optimize for speed over binary size
opt-level = 3
# Enable debugging info for profiling
debug = true

[profile.bench]
debug = true
lto = true
codegen-units = 1

# Use faster linker
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

## I/O Optimization {#io-optimization}

### Async I/O Best Practices

Optimize async operations:

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::fs::File;

pub struct OptimizedFileHandler {
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
}

impl OptimizedFileHandler {
    pub fn new() -> Self {
        Self {
            read_buffer: vec![0; 64 * 1024], // 64KB read buffer
            write_buffer: Vec::with_capacity(64 * 1024),
        }
    }

    pub async fn efficient_file_processing(&mut self, file_path: &str) -> Result<String> {
        let file = File::open(file_path).await?;
        let mut reader = BufReader::with_capacity(64 * 1024, file);

        // Read in chunks to minimize system calls
        let mut total_bytes = 0;
        while let Ok(bytes_read) = reader.read(&mut self.read_buffer).await {
            if bytes_read == 0 { break; }

            // Process chunk
            self.process_chunk(&self.read_buffer[..bytes_read]).await?;
            total_bytes += bytes_read;
        }

        Ok(format!("Processed {} bytes", total_bytes))
    }
}
```

### Connection Pooling

Implement efficient connection management:

```rust
use deadpool_postgres::{Config, Pool, Runtime};
use tokio_postgres::NoTls;

pub struct DatabaseTool {
    pool: Pool,
}

impl DatabaseTool {
    pub async fn new(database_url: &str) -> Result<Self> {
        let mut cfg = Config::new();
        cfg.host = Some("localhost".to_string());
        cfg.user = Some("user".to_string());
        cfg.password = Some("password".to_string());
        cfg.dbname = Some("mydb".to_string());

        // Configure pool for optimal performance
        cfg.pool = Some(deadpool_postgres::PoolConfig {
            max_size: 16,
            timeouts: deadpool_postgres::Timeouts {
                wait: Some(Duration::from_secs(5)),
                create: Some(Duration::from_secs(5)),
                recycle: Some(Duration::from_secs(5)),
            },
            ..Default::default()
        });

        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;

        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl ToolExecutor for DatabaseTool {
    async fn execute(&self, arguments: Option<Value>) -> Result<ToolsCallResponse> {
        let client = self.pool.get().await?;

        // Use prepared statements for better performance
        let stmt = client.prepare_cached("SELECT * FROM users WHERE id = $1").await?;
        let rows = client.query(&stmt, &[&user_id]).await?;

        // Process results...
        Ok(ToolsCallResponse::success(vec![]))
    }
}
```

## Transport Layer Optimization {#transport-optimization}

### WebSocket Optimization

Optimize WebSocket transport:

```rust
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};
use futures::{SinkExt, StreamExt};

pub struct OptimizedWebSocketTransport {
    ws_stream: WebSocketStream<TcpStream>,
    send_buffer: Vec<u8>,
    compression_enabled: bool,
}

impl OptimizedWebSocketTransport {
    pub async fn new(url: &str) -> Result<Self> {
        let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;

        Ok(Self {
            ws_stream,
            send_buffer: Vec::with_capacity(8192),
            compression_enabled: true,
        })
    }

    pub async fn send_optimized(&mut self, data: &[u8]) -> Result<()> {
        let message = if self.compression_enabled && data.len() > 1024 {
            // Use compression for large messages
            let compressed = self.compress_data(data)?;
            Message::Binary(compressed)
        } else {
            Message::Binary(data.to_vec())
        };

        self.ws_stream.send(message).await?;
        Ok(())
    }

    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        Ok(encoder.finish()?)
    }
}
```

### HTTP/2 Support

Use HTTP/2 for better performance:

```rust
use h2::client;
use tokio::net::TcpStream;

pub struct Http2Transport {
    client: client::SendRequest<Bytes>,
}

impl Http2Transport {
    pub async fn new(addr: &str) -> Result<Self> {
        let tcp = TcpStream::connect(addr).await?;
        let (client, connection) = client::handshake(tcp).await?;

        // Spawn the connection task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("HTTP/2 connection error: {}", e);
            }
        });

        Ok(Self { client })
    }

    pub async fn send_request(&mut self, data: Bytes) -> Result<Bytes> {
        let request = http::Request::builder()
            .method(http::Method::POST)
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(())
            .unwrap();

        let (response, mut send_stream) = self.client.send_request(request, false)?;
        send_stream.send_data(data, true).await?;

        let response = response.await?;
        let mut body = response.into_body();
        let mut data = BytesMut::new();

        while let Some(chunk) = body.data().await {
            let chunk = chunk?;
            data.extend_from_slice(&chunk);
        }

        Ok(data.freeze())
    }
}
```

## Scaling Strategies {#scaling}

### Horizontal Scaling

Deploy multiple server instances:

```rust
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct LoadBalancer {
    servers: Vec<String>,
    current_index: AtomicUsize,
    health_checker: HealthChecker,
}

impl LoadBalancer {
    pub fn new(servers: Vec<String>) -> Self {
        Self {
            servers,
            current_index: AtomicUsize::new(0),
            health_checker: HealthChecker::new(),
        }
    }

    pub async fn route_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let healthy_servers = self.health_checker.get_healthy_servers().await;

        if healthy_servers.is_empty() {
            return Err(Error::service_unavailable("No healthy servers available"));
        }

        // Round-robin load balancing
        let index = self.current_index.fetch_add(1, Ordering::Relaxed) % healthy_servers.len();
        let server_url = &healthy_servers[index];

        // Send request to selected server
        self.send_to_server(server_url, request).await
    }
}
```

### Vertical Scaling

Optimize resource usage:

```rust
pub struct ResourceManager {
    cpu_monitor: CpuMonitor,
    memory_monitor: MemoryMonitor,
    auto_scaling_enabled: bool,
}

impl ResourceManager {
    pub async fn monitor_and_adjust(&self) {
        loop {
            let cpu_usage = self.cpu_monitor.get_usage().await;
            let memory_usage = self.memory_monitor.get_usage().await;

            if cpu_usage > 0.8 && self.auto_scaling_enabled {
                self.increase_worker_threads().await;
            } else if cpu_usage < 0.3 {
                self.decrease_worker_threads().await;
            }

            if memory_usage > 0.9 {
                self.trigger_garbage_collection().await;
            }

            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }
}
```

## Monitoring and Observability {#monitoring}

### Performance Metrics

Implement comprehensive metrics:

```rust
use prometheus::{Counter, Histogram, Gauge};

#[derive(Clone)]
pub struct PerformanceMetrics {
    pub request_count: Counter,
    pub request_duration: Histogram,
    pub active_connections: Gauge,
    pub memory_usage: Gauge,
    pub cpu_usage: Gauge,
}

impl PerformanceMetrics {
    pub fn new() -> Result<Self> {
        Ok(Self {
            request_count: Counter::new("mcp_requests_total", "Total requests processed")?,
            request_duration: Histogram::with_opts(
                HistogramOpts::new("mcp_request_duration_seconds", "Request processing time")
                    .buckets(vec![0.001, 0.01, 0.1, 1.0, 5.0, 10.0])
            )?,
            active_connections: Gauge::new("mcp_active_connections", "Active connections")?,
            memory_usage: Gauge::new("mcp_memory_usage_bytes", "Memory usage")?,
            cpu_usage: Gauge::new("mcp_cpu_usage_percent", "CPU usage percentage")?,
        })
    }

    pub async fn update_system_metrics(&self) {
        if let Ok(memory) = sys_info::mem_info() {
            let used_memory = (memory.total - memory.avail) * 1024;
            self.memory_usage.set(used_memory as f64);
        }

        if let Ok(load) = sys_info::loadavg() {
            self.cpu_usage.set(load.one as f64);
        }
    }
}

// Middleware to collect metrics
#[derive(Clone)]
pub struct MetricsMiddleware {
    metrics: Arc<PerformanceMetrics>,
}

#[async_trait::async_trait]
impl RequestMiddleware for MetricsMiddleware {
    async fn before_request(&self, request: &mut JsonRpcRequest) -> Result<()> {
        self.metrics.request_count.inc();
        request.set_metadata("start_time", Instant::now());
        Ok(())
    }

    async fn after_request(&self, request: &JsonRpcRequest, _response: &mut JsonRpcResponse) -> Result<()> {
        if let Some(start_time) = request.get_metadata::<Instant>("start_time") {
            let duration = start_time.elapsed();
            self.metrics.request_duration.observe(duration.as_secs_f64());
        }
        Ok(())
    }
}
```

### Distributed Tracing

Implement distributed tracing:

```rust
use opentelemetry::{trace::Tracer, global};
use tracing_opentelemetry::OpenTelemetryLayer;

pub fn setup_tracing() -> Result<()> {
    global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());
    let tracer = opentelemetry_jaeger::new_collector_pipeline()
        .with_service_name("mcp-server")
        .with_endpoint("http://localhost:14268/api/traces")
        .install_batch(opentelemetry::runtime::Tokio)?;

    let telemetry = OpenTelemetryLayer::new(tracer);

    tracing_subscriber::registry()
        .with(telemetry)
        .with(EnvFilter::from_default_env())
        .init();

    Ok(())
}

#[instrument(skip(self))]
async fn process_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
    let span = tracing::Span::current();
    span.record("request.method", &request.method);
    span.record("request.id", &format!("{:?}", request.id));

    let result = self.execute_request(request).await;

    match &result {
        Ok(response) => {
            span.record("response.success", true);
        }
        Err(error) => {
            span.record("response.success", false);
            span.record("error.message", &error.to_string());
        }
    }

    result
}
```

## Performance Testing

Create comprehensive performance tests:

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_throughput() {
        let server = create_optimized_server().await;
        let start = Instant::now();
        let mut handles = vec![];

        // Spawn 100 concurrent requests
        for _ in 0..100 {
            let client = create_client().await;
            let handle = tokio::spawn(async move {
                client.call_tool("benchmark_tool", json!({})).await
            });
            handles.push(handle);
        }

        let results = futures::future::join_all(handles).await;
        let duration = start.elapsed();

        // Verify all requests succeeded
        assert!(results.iter().all(|r| r.is_ok()));

        // Calculate throughput (should be > 1000 RPS)
        let rps = 100.0 / duration.as_secs_f64();
        assert!(rps > 1000.0, "Throughput too low: {} RPS", rps);
    }

    #[tokio::test]
    async fn test_latency() {
        let server = create_optimized_server().await;
        let client = create_client().await;

        let mut latencies = Vec::new();

        for _ in 0..100 {
            let start = Instant::now();
            let _result = client.call_tool("benchmark_tool", json!({})).await.unwrap();
            latencies.push(start.elapsed());
        }

        latencies.sort();
        let p95 = latencies[95];
        let p99 = latencies[99];

        // Verify latency requirements
        assert!(p95 < Duration::from_millis(100), "P95 latency too high: {:?}", p95);
        assert!(p99 < Duration::from_millis(200), "P99 latency too high: {:?}", p99);
    }

    #[tokio::test]
    async fn test_memory_usage() {
        let initial_memory = get_memory_usage();
        let server = create_optimized_server().await;

        // Process 1000 requests
        for _ in 0..1000 {
            let client = create_client().await;
            let _result = client.call_tool("memory_test_tool", json!({})).await.unwrap();
        }

        // Force garbage collection
        #[cfg(feature = "jemalloc")]
        jemalloc_ctl::epoch::advance().unwrap();

        let final_memory = get_memory_usage();
        let memory_growth = final_memory - initial_memory;

        // Memory growth should be reasonable
        assert!(memory_growth < 100 * 1024 * 1024, "Memory usage grew by {} bytes", memory_growth);
    }
}
```

## Conclusion

Performance tuning MoCoPr applications requires a systematic approach:

1. **Measure First**: Always benchmark before optimizing
2. **Profile Regularly**: Use profiling tools to identify bottlenecks
3. **Optimize Systematically**: Focus on the biggest impact areas first
4. **Monitor Continuously**: Use metrics and tracing to track performance in production

Key optimization areas:

- Memory management and allocation patterns
- CPU-intensive operations and parallelization
- I/O operations and connection pooling
- Transport layer efficiency
- Scaling strategies for high load

Remember that premature optimization is the root of all evil - focus on real bottlenecks identified through measurement and profiling.
