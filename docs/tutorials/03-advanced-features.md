# Advanced MCP Features

This tutorial covers advanced features of MoCoPr, building upon the basics from the previous tutorials.

## Table of Contents

1. [Custom Transport Implementation](#custom-transport-implementation)
2. [Middleware and Interceptors](#middleware-and-interceptors)
3. [Advanced Error Handling](#advanced-error-handling)
4. [Resource Subscriptions and Change Notifications](#resource-subscriptions)
5. [Batch Operations](#batch-operations)
6. [Server-to-Server Communication](#server-to-server-communication)
7. [Performance Optimization](#performance-optimization)

## Custom Transport Implementation

While MoCoPr provides built-in support for stdio, WebSocket, and HTTP transports, you might need custom transport layers for specific use cases.

### Implementing a Custom Transport

```rust
use mocopr_core::transport::{Transport, TransportMessage};
use async_trait::async_trait;

pub struct CustomTransport {
    // Custom transport fields
}

#[async_trait]
impl Transport for CustomTransport {
    async fn send(&self, message: TransportMessage) -> mocopr_core::Result<()> {
        // Custom sending logic
        todo!()
    }

    async fn receive(&self) -> mocopr_core::Result<TransportMessage> {
        // Custom receiving logic
        todo!()
    }

    async fn close(&self) -> mocopr_core::Result<()> {
        // Cleanup logic
        Ok(())
    }
}
```

### Integration with Server

```rust
use mocopr_server::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = McpServerBuilder::new()
        .with_info("custom-transport-server", "1.0.0")
        .with_tools()
        .build()?;

    let transport = CustomTransport::new()?;
    server.run_with_transport(transport).await?;
    Ok(())
}
```

## Middleware and Interceptors

Middleware allows you to intercept and modify requests/responses, implement authentication, logging, rate limiting, etc.

### Request Middleware

```rust
use mocopr_server::middleware::*;
use mocopr_core::prelude::*;
use std::time::Instant;

#[derive(Clone)]
pub struct RequestTimingMiddleware;

#[async_trait::async_trait]
impl Middleware for RequestTimingMiddleware {
    async fn before_request(&self, request: &JsonRpcRequest) -> Result<()> {
        // Store start time in request metadata
        tracing::info!("Processing request: {}", request.method);
        Ok(())
    }

    async fn after_response(
        &self,
        request: &JsonRpcRequest,
        response: &JsonRpcResponse,
    ) -> Result<()> {
        tracing::info!("Completed request: {}", request.method);
        Ok(())
    }

    async fn on_error(&self, request: &JsonRpcRequest, error: &Error) -> Result<()> {
        tracing::error!("Error processing {}: {}", request.method, error);
        Ok(())
    }
}
```

### Authentication Middleware

```rust
use std::collections::HashSet;

#[derive(Clone)]
pub struct AuthenticationMiddleware {
    valid_tokens: HashSet<String>,
}

impl AuthenticationMiddleware {
    pub fn new(valid_tokens: Vec<String>) -> Self {
        Self {
            valid_tokens: valid_tokens.into_iter().collect(),
        }
    }
}

#[async_trait::async_trait]
impl Middleware for AuthenticationMiddleware {
    async fn before_request(&self, request: &JsonRpcRequest) -> Result<()> {
        // Check for authentication in request params
        if let Some(params) = &request.params {
            if let Some(auth) = params.get("auth") {
                if let Some(token) = auth.get("token").and_then(|t| t.as_str()) {
                    if self.valid_tokens.contains(token) {
                        return Ok(());
                    }
                }
            }
        }

        Err(Error::security("Authentication required"))
    }

    async fn after_response(
        &self,
        _request: &JsonRpcRequest,
        _response: &JsonRpcResponse,
    ) -> Result<()> {
        Ok(())
    }

    async fn on_error(&self, _request: &JsonRpcRequest, _error: &Error) -> Result<()> {
        Ok(())
    }
}
```

### Using Middleware in Server

```rust
use mocopr_server::prelude::*;
use mocopr_server::middleware::*;

let server = McpServerBuilder::new()
    .with_info("authenticated-server", "1.0.0")
    .with_middleware(Box::new(RequestTimingMiddleware))
    .with_middleware(Box::new(AuthenticationMiddleware::new(vec!["secret-token".to_string()])))
    .with_tools()
    .build()?;
```

## Advanced Error Handling

MoCoPr provides comprehensive error handling capabilities with custom error types and error context.

### Custom Error Types

```rust
use mocopr_core::error::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyServiceError {
    #[error("Database connection failed: {0}")]
    DatabaseError(String),

    #[error("External API error: {status_code}")]
    ApiError { status_code: u16 },

    #[error("Rate limit exceeded: {requests_per_minute}/min")]
    RateLimitExceeded { requests_per_minute: u32 },
}

impl From<MyServiceError> for Error {
    fn from(err: MyServiceError) -> Self {
        match err {
            MyServiceError::DatabaseError(msg) => Error::internal(msg),
            MyServiceError::ApiError { status_code } => {
                Error::external(format!("API error: {}", status_code))
            }
            MyServiceError::RateLimitExceeded { requests_per_minute } => {
                Error::rate_limit(format!("Rate limit: {}/min exceeded", requests_per_minute))
            }
        }
    }
}
```

### Error Context and Structured Errors

```rust
#[derive(Tool)]
#[tool(name = "database_query", description = "Query the database")]
pub struct DatabaseQueryTool {
    db_pool: Arc<DatabasePool>,
}

#[async_trait::async_trait]
impl ToolExecutor for DatabaseQueryTool {
    async fn execute(&self, arguments: Option<Value>) -> Result<ToolsCallResponse> {
        let query = arguments
            .and_then(|a| a.get("query").and_then(|q| q.as_str()))
            .ok_or_else(|| Error::validation("Missing query parameter"))?;

        // Add error context for better debugging
        let results = self.db_pool
            .execute_query(query)
            .await
            .with_context(|| format!("Failed to execute query: {}", query))
            .map_err(|e| Error::database(e.to_string()))?;

        Ok(ToolsCallResponse::success(vec![Content::Text(
            TextContent::new(&serde_json::to_string(&results)?)
        )]))
    }
}
```

## Resource Subscriptions and Change Notifications {#resource-subscriptions}

MCP supports subscribing to resource changes and receiving real-time notifications.

### Implementing Resource with Change Notifications

```rust
use mocopr_core::prelude::*;
use tokio::sync::broadcast;

#[derive(Resource)]
#[resource(name = "live_data", description = "Real-time data resource")]
pub struct LiveDataResource {
    change_notifier: broadcast::Sender<ResourceChange>,
}

impl LiveDataResource {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { change_notifier: tx }
    }

    // Method to notify subscribers of changes
    pub async fn notify_change(&self, uri: &str) {
        let change = ResourceChange {
            uri: uri.to_string(),
            change_type: ResourceChangeType::Updated,
            timestamp: Utc::now(),
        };

        let _ = self.change_notifier.send(change);
    }
}

#[async_trait::async_trait]
impl ResourceHandler for LiveDataResource {
    async fn read(&self, uri: &str) -> Result<ResourcesReadResponse> {
        // Read current data
        let data = self.fetch_current_data(uri).await?;

        Ok(ResourcesReadResponse {
            contents: vec![ResourceContents::Text(TextResourceContents {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: serde_json::to_string(&data)?,
            })],
            meta: ResponseMetadata::default(),
        })
    }

    async fn subscribe(&self, uri: &str) -> Result<ResourceSubscription> {
        let mut receiver = self.change_notifier.subscribe();

        Ok(ResourceSubscription {
            uri: uri.to_string(),
            change_stream: Box::pin(async_stream::stream! {
                while let Ok(change) = receiver.recv().await {
                    if change.uri == uri {
                        yield change;
                    }
                }
            }),
        })
    }
}
```

## Batch Operations

MCP supports batch requests for improved efficiency when performing multiple operations.

### Implementing Batch-Aware Tools

```rust
#[derive(Tool)]
#[tool(name = "batch_calculator", description = "Calculator supporting batch operations")]
pub struct BatchCalculatorTool;

#[async_trait::async_trait]
impl ToolExecutor for BatchCalculatorTool {
    async fn execute(&self, arguments: Option<Value>) -> Result<ToolsCallResponse> {
        let args = arguments.unwrap_or_default();

        // Check if this is a batch operation
        if let Some(operations) = args.get("batch_operations") {
            return self.execute_batch(operations).await;
        }

        // Handle single operation
        self.execute_single(&args).await
    }
}

impl BatchCalculatorTool {
    async fn execute_batch(&self, operations: &Value) -> Result<ToolsCallResponse> {
        let ops: Vec<Value> = serde_json::from_value(operations.clone())?;
        let mut results = Vec::new();

        // Process operations concurrently
        let tasks: Vec<_> = ops.into_iter()
            .map(|op| self.execute_single(&op))
            .collect();

        let task_results = futures::future::join_all(tasks).await;

        for result in task_results {
            match result {
                Ok(response) => results.push(response),
                Err(e) => results.push(ToolsCallResponse::error(e.to_string())),
            }
        }

        Ok(ToolsCallResponse::batch(results))
    }

    async fn execute_single(&self, args: &Value) -> Result<ToolsCallResponse> {
        // Single operation logic
        todo!()
    }
}
```

## Server-to-Server Communication

MoCoPr supports server-to-server communication for distributed architectures.

### Client for Server Communication

```rust
use mocopr_client::prelude::*;

pub struct ServerOrchestrator {
    calculation_client: McpClient,
    data_client: McpClient,
}

impl ServerOrchestrator {
    pub async fn new() -> Result<Self> {
        let calculation_client = McpClient::builder()
            .connect_stdio("calculator-server")
            .await?;

        let data_client = McpClient::builder()
            .connect_websocket("ws://data-server:8080/mcp")
            .await?;

        Ok(Self {
            calculation_client,
            data_client,
        })
    }

    pub async fn process_complex_request(&self, request: ComplexRequest) -> Result<ComplexResponse> {
        // 1. Fetch data from data server
        let data = self.data_client
            .call_tool("fetch_dataset", json!({
                "dataset_id": request.dataset_id
            }))
            .await?;

        // 2. Perform calculations using calculation server
        let results = self.calculation_client
            .call_tool("batch_calculate", json!({
                "data": data,
                "operations": request.operations
            }))
            .await?;

        // 3. Store results back to data server
        self.data_client
            .call_tool("store_results", json!({
                "results": results,
                "metadata": request.metadata
            }))
            .await?;

        Ok(ComplexResponse { results })
    }
}
```

## Performance Optimization

### Connection Pooling

```rust
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct ConnectionPool {
    clients: Vec<Arc<McpClient>>,
    semaphore: Arc<Semaphore>,
}

impl ConnectionPool {
    pub async fn new(size: usize, server_addr: &str) -> Result<Self> {
        let mut clients = Vec::with_capacity(size);

        for _ in 0..size {
            let client = Arc::new(
                McpClient::builder()
                    .connect_websocket(server_addr)
                    .await?
            );
            clients.push(client);
        }

        Ok(Self {
            clients,
            semaphore: Arc::new(Semaphore::new(size)),
        })
    }

    pub async fn execute<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&McpClient) -> BoxFuture<'_, Result<T>>,
    {
        let _permit = self.semaphore.acquire().await?;
        let client = &self.clients[rand::random::<usize>() % self.clients.len()];
        f(client).await
    }
}
```

### Caching

```rust
use moka::future::Cache;
use std::time::Duration;

#[derive(Tool)]
#[tool(name = "cached_tool", description = "Tool with caching support")]
pub struct CachedTool {
    cache: Cache<String, Value>,
}

impl CachedTool {
    pub fn new() -> Self {
        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(300)) // 5 minutes
            .max_capacity(1000)
            .build();

        Self { cache }
    }
}

#[async_trait::async_trait]
impl ToolExecutor for CachedTool {
    async fn execute(&self, arguments: Option<Value>) -> Result<ToolsCallResponse> {
        let args = arguments.unwrap_or_default();
        let cache_key = self.generate_cache_key(&args);

        // Check cache first
        if let Some(cached_result) = self.cache.get(&cache_key).await {
            return Ok(ToolsCallResponse::success(vec![
                Content::Text(TextContent::new(&cached_result.to_string()))
            ]));
        }

        // Execute expensive operation
        let result = self.expensive_operation(&args).await?;

        // Cache the result
        self.cache.insert(cache_key, result.clone()).await;

        Ok(ToolsCallResponse::success(vec![
            Content::Text(TextContent::new(&result.to_string()))
        ]))
    }
}
```

### Streaming Responses

```rust
use futures::Stream;
use async_stream::stream;

#[derive(Tool)]
#[tool(name = "streaming_tool", description = "Tool with streaming response")]
pub struct StreamingTool;

#[async_trait::async_trait]
impl ToolExecutor for StreamingTool {
    async fn execute(&self, arguments: Option<Value>) -> Result<ToolsCallResponse> {
        let args = arguments.unwrap_or_default();

        // Return streaming response for large datasets
        let stream = self.create_data_stream(&args);

        Ok(ToolsCallResponse::stream(stream))
    }
}

impl StreamingTool {
    fn create_data_stream(&self, args: &Value) -> impl Stream<Item = Content> {
        let chunk_size = args.get("chunk_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(1000) as usize;

        stream! {
            for chunk_id in 0..100 { // 100 chunks
                let chunk_data = self.generate_chunk(chunk_id, chunk_size).await;
                yield Content::Text(TextContent::new(&chunk_data));

                // Small delay to prevent overwhelming the client
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }
}
```

## Testing Advanced Features

### Integration Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mocopr_core::test_utils::*;

    #[tokio::test]
    async fn test_middleware_chain() {
        let server = McpServerBuilder::new()
            .with_info("test-server", "1.0.0")
            .with_middleware(RequestTimingMiddleware)
            .with_middleware(AuthenticationMiddleware::new(vec!["test-token".to_string()]))
            .with_tools()
            .with_tool(TestTool)
            .build()
            .unwrap();

        let mut client = TestClient::new(server).await;

        // Test authenticated request
        let response = client
            .with_header("Authorization", "Bearer test-token")
            .call_tool("test_tool", json!({}))
            .await
            .unwrap();

        assert!(response.is_success());
        assert!(response.metadata().contains_key("processing_time_ms"));

        // Test unauthenticated request
        let response = client
            .call_tool("test_tool", json!({}))
            .await;

        assert!(response.is_err());
    }

    #[tokio::test]
    async fn test_resource_subscriptions() {
        let resource = Arc::new(LiveDataResource::new());
        let subscription = resource.subscribe("test://data/live").await.unwrap();

        // Trigger a change
        resource.notify_change("test://data/live").await;

        // Verify subscription receives the change
        let change = subscription.next_change().await.unwrap();
        assert_eq!(change.uri, "test://data/live");
        assert_eq!(change.change_type, ResourceChangeType::Updated);
    }
}
```

## Conclusion

This tutorial covered advanced MoCoPr features including:

- Custom transport implementations
- Middleware and request interceptors
- Advanced error handling with custom error types
- Resource subscriptions and change notifications
- Batch operations for improved efficiency
- Server-to-server communication patterns
- Performance optimization techniques

These features enable you to build sophisticated, production-ready MCP servers that can handle complex use cases and scale to meet demanding requirements.

## Next Steps

- Read the [Performance Tuning Guide](05-performance-tuning.md)
- Explore the [Architecture Guide](../guides/architecture.md)
- Check out more [Examples](../../examples/)
- Join our community discussions
