# Troubleshooting Guide

This guide helps diagnose and resolve common issues when working with MoCoPr.

## Table of Contents

1. [Common Issues](#common-issues)
2. [Build and Compilation Issues](#build-issues)
3. [Runtime Errors](#runtime-errors)
4. [Performance Problems](#performance-problems)
5. [Transport Issues](#transport-issues)
6. [Protocol Errors](#protocol-errors)
7. [Debugging Techniques](#debugging-techniques)
8. [Getting Help](#getting-help)

## Common Issues {#common-issues}

### Server Won't Start

**Symptoms:**

- Server exits immediately
- "Address already in use" error
- Permission denied errors

**Solutions:**

1. **Check if port is already in use:**

```bash
# On Windows
netstat -an | findstr :8080

# On Linux/macOS
lsof -i :8080
```

2. **Run with different port:**

```rust
let server = McpServerBuilder::new()
    .with_info("my-server", "1.0.0")
    .bind("127.0.0.1:8081")  // Use different port
    .build()?;
```

3. **Check permissions for privileged ports:**

```bash
# Ports below 1024 require admin/root privileges
sudo ./your-server  # Linux/macOS
# Run as Administrator on Windows
```

### Client Connection Failures

**Symptoms:**

- "Connection refused" errors
- Timeouts during connection
- SSL/TLS handshake failures

**Solutions:**

1. **Verify server is running:**

```bash
# Test with curl
curl http://localhost:8080/health

# Test WebSocket connection
wscat -c ws://localhost:8080/mcp
```

2. **Check firewall settings:**

```bash
# Windows Firewall
netsh advfirewall firewall add rule name="MCP Server" dir=in action=allow protocol=TCP localport=8080

# Linux (ufw)
sudo ufw allow 8080

# macOS
# Check System Preferences > Security & Privacy > Firewall
```

3. **SSL/TLS issues:**

```rust
// Disable SSL verification for testing (NOT for production)
let client = McpClient::builder()
    .danger_accept_invalid_certs(true)
    .connect("wss://localhost:8080/mcp")
    .await?;
```

### Tool Not Found Errors

**Symptoms:**

- "Tool 'xyz' not found" error
- Tool list is empty

**Solutions:**

1. **Verify tool registration:**

```rust
let server = McpServerBuilder::new()
    .with_info("my-server", "1.0.0")
    .with_tools()  // Enable tools capability
    .with_tool(MyTool)  // Register the tool
    .build()?;
```

2. **Check tool name:**

```rust
#[derive(Tool)]
#[tool(name = "my_tool", description = "My tool")]  // Name must match
struct MyTool;
```

3. **Debug tool registration:**

```rust
let server = McpServerBuilder::new()
    .with_info("my-server", "1.0.0")
    .with_tools()
    .with_tool(MyTool)
    .build()?;

// List registered tools
let tools = server.list_tools().await?;
println!("Registered tools: {:?}", tools);
```

## Build and Compilation Issues {#build-issues}

### Dependency Resolution Errors

**Error:**

```
error: failed to resolve dependencies
```

**Solutions:**

1. **Clear Cargo cache:**

```bash
cargo clean
rm -rf ~/.cargo/registry
cargo build
```

2. **Update dependencies:**

```bash
cargo update
```

3. **Check for conflicting versions in Cargo.toml:**

```toml
[dependencies]
# Ensure compatible versions
mocopr-core = "0.1.0"
mocopr-server = "0.1.0"  # Same version as core
```

### Macro Compilation Errors

**Error:**

```
error: cannot find derive macro `Tool` in this scope
```

**Solutions:**

1. **Add mocopr-macros dependency:**

```toml
[dependencies]
mocopr-macros = "0.1.0"
```

2. **Import the macro:**

```rust
use mocopr_macros::Tool;

#[derive(Tool)]
#[tool(name = "my_tool", description = "Description")]
struct MyTool;
```

3. **Check macro syntax:**

```rust
// Correct syntax
#[derive(Tool)]
#[tool(name = "my_tool", description = "My tool")]
struct MyTool;

// Incorrect - missing description
#[derive(Tool)]
#[tool(name = "my_tool")]  // This will fail
struct MyTool;
```

### Async/Await Issues

**Error:**

```
error: `async fn` is not permitted in the current context
```

**Solutions:**

1. **Add async-trait dependency:**

```toml
[dependencies]
async-trait = "0.1"
```

2. **Use async-trait attribute:**

```rust
use async_trait::async_trait;

#[async_trait]
impl ToolExecutor for MyTool {
    async fn execute(&self, args: Option<Value>) -> Result<ToolsCallResponse> {
        // Implementation
    }
}
```

## Runtime Errors {#runtime-errors}

### JSON-RPC Protocol Errors

**Error:**

```
JsonRpcError { code: -32600, message: "Invalid Request" }
```

**Solutions:**

1. **Verify request format:**

```rust
// Correct JSON-RPC 2.0 format
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
        "name": "my_tool",
        "arguments": {}
    }
}
```

2. **Check method names:**

```rust
// Valid MCP methods
"initialize"
"tools/list"
"tools/call"
"resources/list"
"resources/read"
"prompts/list"
"prompts/get"
```

### Serialization/Deserialization Errors

**Error:**

```
Error: missing field `name` at line 1 column 123
```

**Solutions:**

1. **Check struct field names:**

```rust
#[derive(Serialize, Deserialize)]
struct ToolCallParams {
    pub name: String,        // Required field
    pub arguments: Option<Value>,
}
```

2. **Use proper serde attributes:**

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]  // For JavaScript compatibility
struct MyStruct {
    #[serde(skip_serializing_if = "Option::is_none")]
    optional_field: Option<String>,
}
```

### Memory and Resource Issues

**Error:**

```
thread 'main' panicked at 'out of memory'
```

**Solutions:**

1. **Implement resource limits:**

```rust
use tokio::sync::Semaphore;

struct RateLimitedTool {
    semaphore: Semaphore,
}

impl RateLimitedTool {
    pub fn new() -> Self {
        Self {
            semaphore: Semaphore::new(10),  // Max 10 concurrent operations
        }
    }
}

#[async_trait]
impl ToolExecutor for RateLimitedTool {
    async fn execute(&self, args: Option<Value>) -> Result<ToolsCallResponse> {
        let _permit = self.semaphore.acquire().await?;
        // Do work with resource limits
        Ok(ToolsCallResponse::success(vec![]))
    }
}
```

2. **Monitor memory usage:**

```rust
// Add memory monitoring
let memory_usage = sys_info::mem_info()?;
if memory_usage.free < 100_000 {  // Less than 100MB free
    return Err(Error::resource_exhausted("Low memory"));
}
```

## Performance Problems {#performance-problems}

### Slow Response Times

**Symptoms:**

- Requests taking > 1 second
- Client timeouts
- High CPU usage

**Debugging:**

1. **Add timing instrumentation:**

```rust
use std::time::Instant;

#[async_trait]
impl ToolExecutor for MyTool {
    async fn execute(&self, args: Option<Value>) -> Result<ToolsCallResponse> {
        let start = Instant::now();

        let result = self.do_work(args).await?;

        let duration = start.elapsed();
        if duration > Duration::from_millis(100) {
            tracing::warn!("Slow tool execution: {:?}", duration);
        }

        Ok(result)
    }
}
```

2. **Profile with cargo flamegraph:**

```bash
cargo install flamegraph
cargo flamegraph --bin your-server
```

3. **Use async profiling:**

```bash
cargo install tokio-console
# Add to Cargo.toml:
# tokio = { version = "1", features = ["tracing"] }
# console-subscriber = "0.1"

# In your code:
console_subscriber::init();

# Run with:
tokio-console
```

### High Memory Usage

**Symptoms:**

- Memory usage keeps growing
- Out of memory errors
- Slow garbage collection

**Solutions:**

1. **Use memory profiling:**

```bash
# Install heaptrack (Linux)
heaptrack ./target/release/your-server

# Or use valgrind
valgrind --tool=massif ./target/release/your-server
```

2. **Implement object pooling:**

```rust
use object_pool::Pool;

lazy_static! {
    static ref BUFFER_POOL: Pool<Vec<u8>> = Pool::new(32, || {
        Vec::with_capacity(1024)
    });
}

fn get_buffer() -> object_pool::Reusable<Vec<u8>> {
    BUFFER_POOL.try_pull().unwrap_or_else(|| {
        BUFFER_POOL.attach(Vec::with_capacity(1024))
    })
}
```

## Transport Issues {#transport-issues}

### WebSocket Connection Problems

**Error:**

```
WebSocket handshake failed
```

**Solutions:**

1. **Check WebSocket headers:**

```rust
// Server setup
use tokio_tungstenite::{accept_async, tungstenite::Message};

async fn handle_websocket(stream: TcpStream) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    // Handle messages
    Ok(())
}
```

2. **Test with wscat:**

```bash
# Install wscat
npm install -g wscat

# Test connection
wscat -c ws://localhost:8080/mcp

# Send test message
> {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
```

### Stdio Transport Issues

**Symptoms:**

- No response from server
- Broken pipe errors
- Input/output formatting issues

**Solutions:**

1. **Test with manual input:**

```bash
# Start server
./your-server

# Send JSON-RPC request via stdin
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | ./your-server
```

2. **Check line buffering:**

```rust
use std::io::{self, BufRead, Write};

// Ensure output is flushed
io::stdout().flush()?;
```

3. **Debug with verbose logging:**

```rust
// Enable debug logging
env_logger::init();

// Or use tracing
tracing_subscriber::fmt()
    .with_env_filter("debug")
    .init();
```

## Protocol Errors {#protocol-errors}

### Capability Negotiation Failures

**Error:**

```
Server capabilities mismatch
```

**Solutions:**

1. **Check server capabilities:**

```rust
let server = McpServerBuilder::new()
    .with_info("my-server", "1.0.0")
    .with_tools()      // Enable tools
    .with_resources()  // Enable resources
    .with_prompts()    // Enable prompts
    .build()?;
```

2. **Verify client expectations:**

```rust
let client = McpClient::builder()
    .expect_tools(true)     // Require tools capability
    .expect_resources(false) // Resources optional
    .connect("ws://localhost:8080/mcp")
    .await?;
```

### Message Format Errors

**Error:**

```
Invalid message format
```

**Solutions:**

1. **Validate JSON structure:**

```bash
# Use jq to validate JSON
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | jq .
```

2. **Check message framing:**

```rust
// For WebSocket transport
use tokio_tungstenite::tungstenite::Message;

// Send as text message
ws_stream.send(Message::Text(json_string)).await?;

// For HTTP transport
use reqwest;

let response = reqwest::Client::new()
    .post("http://localhost:8080/mcp")
    .header("Content-Type", "application/json")
    .body(json_string)
    .send()
    .await?;
```

## Debugging Techniques {#debugging-techniques}

### Enable Debug Logging

1. **Set environment variable:**

```bash
export RUST_LOG=debug
./your-server
```

2. **Or in code:**

```rust
tracing_subscriber::fmt()
    .with_env_filter("mocopr=debug,your_crate=debug")
    .init();
```

### Use the Debug Server

MoCoPr includes a debug server for testing:

```bash
cd examples/simple-server
RUST_LOG=debug cargo run
```

### Test with curl

```bash
# Test HTTP transport
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
```

### Packet Capture

Use Wireshark or tcpdump to analyze network traffic:

```bash
# Capture localhost traffic
sudo tcpdump -i lo -w mcp-traffic.pcap port 8080

# Or use Wireshark GUI
wireshark -i lo -f "port 8080"
```

### Integration Testing

Create comprehensive tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mocopr_core::test_utils::*;

    #[tokio::test]
    async fn test_tool_execution() {
        let server = create_test_server().await;
        let mut client = TestClient::new(server).await;

        let response = client
            .call_tool("test_tool", json!({"param": "value"}))
            .await;

        assert!(response.is_ok());
        let result = response.unwrap();
        assert_eq!(result.status, "success");
    }
}
```

## Getting Help {#getting-help}

### Before Asking for Help

1. **Check this troubleshooting guide**
2. **Search existing issues on GitHub**
3. **Enable debug logging and collect logs**
4. **Create a minimal reproduction case**

### Information to Include

When reporting issues, please include:

- **MoCoPr version**: `cargo tree | grep mocopr`
- **Rust version**: `rustc --version`
- **Operating system**: `uname -a` (Linux/macOS) or `ver` (Windows)
- **Error messages**: Full stack traces
- **Configuration**: Relevant code snippets
- **Steps to reproduce**: Minimal example

### Where to Get Help

1. **GitHub Issues**: <https://github.com/yourusername/MoCoPr/issues>
2. **Discussions**: <https://github.com/yourusername/MoCoPr/discussions>
3. **Discord**: [Community Discord Server]
4. **Stack Overflow**: Tag questions with `mocopr` and `rust`

### Contributing Bug Fixes

If you find and fix a bug:

1. **Create a test case** that reproduces the issue
2. **Implement the fix** with proper error handling
3. **Add documentation** explaining the fix
4. **Submit a pull request** with clear description

Example bug fix:

```rust
// Before: Potential panic
let value = args["required_param"].as_str().unwrap();

// After: Proper error handling
let value = args
    .get("required_param")
    .and_then(|v| v.as_str())
    .ok_or_else(|| Error::validation("Missing required_param"))?;
```

Remember: Good bug reports help everyone, and contributing fixes helps the entire community!
