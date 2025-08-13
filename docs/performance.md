# Performance Optimization Guide

This guide covers performance optimization techniques, benchmarking strategies, and best practices for building high-performance MCP applications with MoCoPr.

## 📊 Performance Overview

MoCoPr is designed for high performance with:

- **Zero-copy Serialization**: Minimized memory allocations
- **Async I/O**: Non-blocking operations throughout
- **Connection Pooling**: Efficient resource management
- **Batching Support**: Reduced overhead for multiple operations
- **Streaming**: Memory-efficient handling of large payloads
- **Caching**: Intelligent caching at multiple levels

## 🔍 Performance Profiling

### Basic Profiling Setup

```rust
use mocopr::profiling::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Enable performance profiling
    let profiler = Profiler::builder()
        .with_cpu_profiling(true)
        .with_memory_profiling(true)
        .with_request_tracing(true)
        .build();

    let server = McpServer::builder()
        .with_profiler(profiler)
        .build()?;
    
    server.run_stdio().await?;
    Ok(())
}
```

### Advanced Profiling with Custom Metrics

```rust
use mocopr::metrics::*;

let metrics = MetricsCollector::builder()
    .with_histogram("request_duration", &[0.1, 0.5, 1.0, 5.0])
    .with_counter("requests_total")
    .with_gauge("active_connections")
    .build();

let server = McpServer::builder()
    .with_metrics(metrics)
    .with_instrumentation(true)
    .build()?;
```

### Flamegraph Generation

```bash
# Install cargo-flamegraph
cargo install flamegraph

# Generate flamegraph for server
cargo flamegraph --example production-server

# Generate flamegraph for specific test
cargo flamegraph --test stress_tests -- test_high_volume_requests
```

## ⚡ Optimization Techniques

### Memory Optimization

#### Efficient String Handling

```rust
use std::borrow::Cow;

// Use Cow for potentially borrowed strings
pub fn process_message<'a>(input: &'a str) -> Cow<'a, str> {
    if needs_processing(input) {
        Cow::Owned(process_string(input))
    } else {
        Cow::Borrowed(input)
    }
}

// Use &str in function parameters when possible
pub fn validate_method_name(method: &str) -> bool {
    // Instead of taking String
    !method.is_empty() && method.len() < MAX_METHOD_LENGTH
}
```

#### Object Pooling for Frequent Allocations

```rust
use object_pool::Pool;

struct ResponsePool {
    pool: Pool<JsonRpcResponse>,
}

impl ResponsePool {
    fn new() -> Self {
        Self {
            pool: Pool::new(32, || JsonRpcResponse::default()),
        }
    }
    
    fn get_response(&self) -> PoolGuard<JsonRpcResponse> {
        self.pool.try_pull().unwrap_or_else(|| {
            self.pool.pull()
        })
    }
}
```

#### Small Vector Optimization

```rust
use smallvec::{SmallVec, smallvec};

// Use SmallVec for collections that are usually small
type ParameterList = SmallVec<[Parameter; 4]>;
type HeaderList = SmallVec<[Header; 8]>;

fn collect_parameters() -> ParameterList {
    let mut params = smallvec![];
    // Most tool calls have few parameters
    params.push(Parameter::new("input", "string"));
    params
}
```

### Async Performance

#### Connection Pooling

```rust
use mocopr::transport::pool::*;

let connection_pool = ConnectionPool::builder()
    .with_max_connections(100)
    .with_min_idle_connections(10)
    .with_connection_timeout(Duration::from_secs(30))
    .with_idle_timeout(Duration::from_secs(300))
    .with_health_check_interval(Duration::from_secs(60))
    .build();

let client = McpClient::builder()
    .with_connection_pool(connection_pool)
    .build();
```

#### Batching Operations

```rust
use mocopr::batch::*;

let batch_processor = BatchProcessor::builder()
    .with_max_batch_size(100)
    .with_batch_timeout(Duration::from_millis(10))
    .with_concurrency_limit(10)
    .build();

// Batch multiple tool calls
let results = batch_processor.execute_batch(&[
    ToolCall::new("calculator", json!({"op": "add", "a": 1, "b": 2})),
    ToolCall::new("calculator", json!({"op": "mul", "a": 3, "b": 4})),
    ToolCall::new("validator", json!({"input": "test"})),
]).await?;
```

#### Streaming for Large Payloads

```rust
use futures::stream::StreamExt;

async fn stream_large_resource(uri: &str) -> Result<impl Stream<Item = Result<Bytes>>> {
    let stream = ResourceStream::open(uri).await?;
    
    Ok(stream.map(|chunk| {
        chunk.map_err(Into::into)
    }))
}

// Use streaming in handlers
async fn handle_large_resource(uri: &str) -> Result<ResourceResponse> {
    if is_large_resource(uri) {
        Ok(ResourceResponse::Stream(stream_large_resource(uri).await?))
    } else {
        Ok(ResourceResponse::Content(load_resource(uri).await?))
    }
}
```

### Serialization Performance

#### Efficient JSON Handling

```rust
use simd_json::prelude::*;

// Use simd-json for better performance on large payloads
fn fast_json_parse(input: &mut [u8]) -> Result<simd_json::OwnedValue> {
    simd_json::to_owned_value(input).map_err(Into::into)
}

// Pre-allocate string buffers
fn serialize_with_capacity<T: Serialize>(value: &T, estimated_size: usize) -> Result<String> {
    let mut buffer = String::with_capacity(estimated_size);
    let mut serializer = serde_json::Serializer::new(unsafe {
        buffer.as_mut_vec()
    });
    value.serialize(&mut serializer)?;
    Ok(buffer)
}
```

#### Binary Serialization for Internal Communication

```rust
use bincode;

// Use binary format for internal server-to-server communication
fn serialize_binary<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    bincode::serialize(value).map_err(Into::into)
}

fn deserialize_binary<T: DeserializeOwned>(data: &[u8]) -> Result<T> {
    bincode::deserialize(data).map_err(Into::into)
}
```

### Caching Strategies

#### Multi-Level Caching

```rust
use mocopr::cache::*;

let cache_config = CacheConfig::builder()
    .with_l1_cache(LruCache::new(1000))  // In-memory cache
    .with_l2_cache(RedisCache::new("redis://localhost"))  // Shared cache
    .with_ttl_default(Duration::from_secs(300))
    .with_compression(CompressionType::Lz4)
    .build();

let server = McpServer::builder()
    .with_cache(cache_config)
    .build()?;
```

#### Smart Cache Keys

```rust
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

fn generate_cache_key(method: &str, params: &Value) -> String {
    let mut hasher = DefaultHasher::new();
    method.hash(&mut hasher);
    
    // Normalize parameters for consistent hashing
    let normalized_params = normalize_json(params);
    normalized_params.to_string().hash(&mut hasher);
    
    format!("mcp:{}:{:x}", method, hasher.finish())
}

fn normalize_json(value: &Value) -> Value {
    // Sort object keys for consistent hashing
    match value {
        Value::Object(map) => {
            let mut sorted: BTreeMap<String, Value> = map.iter()
                .map(|(k, v)| (k.clone(), normalize_json(v)))
                .collect();
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(arr) => {
            Value::Array(arr.iter().map(normalize_json).collect())
        }
        _ => value.clone(),
    }
}
```

## 📈 Benchmarking

### Comprehensive Benchmark Suite

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};

fn bench_message_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_processing");
    
    // Test different message sizes
    for size in [1_024, 10_240, 102_400].iter() {
        let message = create_test_message(*size);
        
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &message,
            |b, msg| {
                b.iter(|| serialize_message(msg));
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("deserialize", size),
            &serde_json::to_string(&message).unwrap(),
            |b, json| {
                b.iter(|| deserialize_message(json));
            },
        );
    }
    
    group.finish();
}

fn bench_concurrent_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_processing");
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    for concurrency in [1, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_requests", concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async move {
                    let tasks: Vec<_> = (0..concurrency)
                        .map(|_| process_request_async())
                        .collect();
                    
                    futures::future::join_all(tasks).await;
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(benches, bench_message_processing, bench_concurrent_processing);
criterion_main!(benches);
```

### Real-World Load Testing

```rust
use mocopr::testing::*;

#[tokio::test]
async fn load_test_realistic_scenario() -> Result<()> {
    let server = setup_test_server().await?;
    let load_tester = LoadTester::builder()
        .with_concurrent_clients(50)
        .with_requests_per_second(1000)
        .with_duration(Duration::from_secs(60))
        .with_ramp_up(Duration::from_secs(10))
        .build();
    
    let scenario = LoadScenario::builder()
        .with_operation("list_tools", 0.1)  // 10% of requests
        .with_operation("call_tool", 0.7)   // 70% of requests
        .with_operation("read_resource", 0.2) // 20% of requests
        .build();
    
    let results = load_tester.run(scenario).await?;
    
    assert!(results.avg_latency < Duration::from_millis(100));
    assert!(results.p99_latency < Duration::from_millis(500));
    assert!(results.error_rate < 0.01); // Less than 1% errors
    assert!(results.throughput > 950.0); // At least 95% of target RPS
    
    Ok(())
}
```

### Memory Leak Detection

```rust
#[tokio::test]
async fn test_memory_stability() -> Result<()> {
    let initial_memory = get_memory_usage();
    
    // Run for extended period
    for iteration in 0..10_000 {
        let _result = process_typical_request().await?;
        
        if iteration % 1000 == 0 {
            let current_memory = get_memory_usage();
            let growth = current_memory - initial_memory;
            
            // Allow some growth but not unbounded
            assert!(growth < 50_000_000, "Memory grew by {} bytes", growth);
            
            // Force garbage collection
            drop(_result);
            tokio::task::yield_now().await;
        }
    }
    
    Ok(())
}
```

## 🔧 Configuration Tuning

### Runtime Configuration

```rust
use mocopr::config::*;

let runtime_config = RuntimeConfig::builder()
    .with_worker_threads(num_cpus::get())
    .with_blocking_threads(512)
    .with_thread_stack_size(2 * 1024 * 1024) // 2MB
    .with_max_blocking_threads(512)
    .with_thread_keep_alive(Duration::from_secs(10))
    .build();

let server = McpServer::builder()
    .with_runtime_config(runtime_config)
    .build()?;
```

### Buffer Size Tuning

```rust
let transport_config = TransportConfig::builder()
    .with_read_buffer_size(64 * 1024)   // 64KB read buffer
    .with_write_buffer_size(64 * 1024)  // 64KB write buffer
    .with_max_frame_size(16 * 1024 * 1024) // 16MB max frame
    .with_compression_threshold(1024)    // Compress frames > 1KB
    .build();
```

### Connection Limits

```rust
let server_config = ServerConfig::builder()
    .with_max_connections(10_000)
    .with_connection_timeout(Duration::from_secs(30))
    .with_keep_alive(Duration::from_secs(75))
    .with_tcp_nodelay(true)
    .with_tcp_keepalive(Some(Duration::from_secs(60)))
    .build();
```

## 📊 Performance Monitoring

### Custom Metrics

```rust
use mocopr::metrics::*;

#[derive(Clone)]
struct CustomMetrics {
    request_duration: HistogramVec,
    cache_hits: CounterVec,
    active_connections: IntGauge,
}

impl CustomMetrics {
    fn new() -> Self {
        Self {
            request_duration: HistogramVec::new(
                HistogramOpts::new("request_duration_seconds", "Request duration"),
                &["method", "status"]
            ).unwrap(),
            cache_hits: CounterVec::new(
                Opts::new("cache_hits_total", "Cache hits"),
                &["cache_type", "hit_miss"]
            ).unwrap(),
            active_connections: IntGauge::new(
                "active_connections", "Number of active connections"
            ).unwrap(),
        }
    }
    
    fn record_request(&self, method: &str, duration: Duration, success: bool) {
        let status = if success { "success" } else { "error" };
        self.request_duration
            .with_label_values(&[method, status])
            .observe(duration.as_secs_f64());
    }
}
```

### Performance Dashboards

```rust
use mocopr::dashboard::*;

let dashboard = PerformanceDashboard::builder()
    .with_metrics_endpoint("/metrics")
    .with_health_endpoint("/health")
    .with_profiling_endpoint("/debug/pprof")
    .with_real_time_charts(true)
    .build();

server.with_dashboard(dashboard);
```

## 🎯 Performance Best Practices

### Do's

✅ **Use appropriate data structures**
```rust
// Use HashMap for O(1) lookups
use std::collections::HashMap;
let mut tool_registry: HashMap<String, ToolHandler> = HashMap::new();

// Use Vec for ordered data
let mut request_queue: Vec<JsonRpcRequest> = Vec::with_capacity(expected_size);

// Use BTreeMap for sorted data
use std::collections::BTreeMap;
let mut sorted_resources: BTreeMap<String, Resource> = BTreeMap::new();
```

✅ **Minimize allocations in hot paths**
```rust
// Pre-allocate collections
let mut buffer = String::with_capacity(estimated_size);

// Reuse allocations
fn process_messages(messages: &[Message], buffer: &mut String) {
    for message in messages {
        buffer.clear();
        serialize_message(message, buffer);
        // Process without reallocating
    }
}
```

✅ **Use streaming for large data**
```rust
async fn stream_large_response() -> Result<impl Stream<Item = Result<Bytes>>> {
    // Instead of loading everything into memory
    Ok(tokio_util::io::ReaderStream::new(file_reader))
}
```

### Don'ts

❌ **Don't block the async runtime**
```rust
// BAD: Blocking call in async context
async fn bad_handler() {
    std::thread::sleep(Duration::from_secs(1)); // Blocks entire runtime!
}

// GOOD: Use async sleep
async fn good_handler() {
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

❌ **Don't ignore backpressure**
```rust
// BAD: Unbounded channel
let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

// GOOD: Bounded channel with backpressure
let (tx, rx) = tokio::sync::mpsc::channel(1000);
```

❌ **Don't create unnecessary futures**
```rust
// BAD: Boxing when not needed
fn bad_async() -> Box<dyn Future<Output = Result<()>>> {
    Box::new(async { Ok(()) })
}

// GOOD: Return impl Future
async fn good_async() -> Result<()> {
    Ok(())
}
```

## 📚 Performance Resources

### Tools and Libraries

- **Profiling**: `cargo-flamegraph`, `perf`, `instruments`
- **Benchmarking**: `criterion`, `iai`
- **Memory**: `valgrind`, `heaptrack`
- **Async**: `tokio-console`, `tracing`

### External Resources

- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Tokio Performance Guide](https://tokio.rs/tokio/topics/performance)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [Async Rust Performance](https://ryhl.io/blog/async-what-is-blocking/)

---

**Remember**: Profile first, optimize second. Don't optimize prematurely, but design with performance in mind.
