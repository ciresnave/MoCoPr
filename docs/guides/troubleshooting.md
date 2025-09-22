# Troubleshooting

This guide provides solutions to common problems and errors you might encounter when using MoCoPr.

## Table of Contents

*   **1. Build and Compilation Errors**
    *   `error[E0432]: unresolved import`
    *   `error[E0308]: mismatched types`
    *   `error[E0599]: no method named ...`
*   **2. Runtime Errors**
    *   `Connection refused`
    *   `Timeout`
    *   `Parse error`
*   **3. Performance Issues**
    *   Slow response times
    *   High memory usage

## 1. Build and Compilation Errors

### `error[E0432]: unresolved import`

This error occurs when you try to use a type or trait that is not in scope. Make sure you have added the necessary `use` statements at the top of your file.

**Example:**

```rust
// This will fail to compile because `McpServerBuilder` is not in scope
let server = McpServerBuilder::new().build()?;
```

**Solution:**

```rust
use mocopr::prelude::*;

// This will compile successfully
let server = McpServerBuilder::new().build()?;
```

### `error[E0308]: mismatched types`

This error occurs when the compiler expects a certain type but finds a different one. Check the types of your variables and function return values to ensure they match.

**Example:**

```rust
// This will fail to compile because `my_function` returns a `String` but `main` expects a `Result<()>`
fn my_function() -> String {
    "hello".to_string()
}

fn main() -> anyhow::Result<()> {
    my_function()?;
    Ok(())
}
```

**Solution:**

```rust
fn my_function() -> anyhow::Result<String> {
    Ok("hello".to_string())
}

fn main() -> anyhow::Result<()> {
    my_function()?;
    Ok(())
}
```

### `error[E0599]: no method named ...`

This error occurs when you try to call a method that does not exist on a type. Make sure you have imported the necessary traits to bring the method into scope.

**Example:**

```rust
// This will fail to compile because the `writer` method is not in scope
let mut buffer = bytes::BytesMut::new();
buffer.writer();
```

**Solution:**

```rust
use bytes::BufMut;

let mut buffer = bytes::BytesMut::new();
buffer.writer();
```

## 2. Runtime Errors

### `Connection refused`

This error occurs when the client is unable to connect to the server. Make sure the server is running and that the address and port are correct.

### `Timeout`

This error occurs when a request takes too long to complete. You can increase the timeout for a request using the `send_request_with_timeout` method on the `Session`.

### `Parse error`

This error occurs when the server or client receives a malformed JSON-RPC message. Check the format of your messages to ensure they are valid.

## 3. Performance Issues

### Slow response times

If your server is responding slowly, consider the following:

*   **Enable `simd-json`:** Use the `simd-json-performance` feature to accelerate JSON parsing.
*   **Use a multi-threaded runtime:** Use the `with_multi_threaded_runtime` method to enable the multi-threaded Tokio runtime.
*   **Use connection pooling:** For tools that make outbound HTTP requests, use a shared `reqwest::Client`.
*   **Benchmark your code:** Use the benchmark suite to identify and optimize performance bottlenecks.

### High memory usage

If your server is using a lot of memory, consider the following:

*   **Optimize serialization:** Use a `BytesMut` buffer to reduce allocations when serializing messages.
*   **Use streaming:** For large resources, use streaming to avoid loading the entire resource into memory.
