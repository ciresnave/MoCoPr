//! Middleware for MCP servers

use mocopr_core::prelude::*;
use tracing::{error, info, warn};

/// Middleware trait for processing requests
#[async_trait::async_trait]
pub trait Middleware: Send + Sync {
    /// Process a request before it reaches the handler
    async fn before_request(&self, request: &JsonRpcRequest) -> Result<()>;

    /// Process a response before it's sent back
    async fn after_response(
        &self,
        request: &JsonRpcRequest,
        response: &JsonRpcResponse,
    ) -> Result<()>;

    /// Handle errors that occur during processing
    async fn on_error(&self, request: &JsonRpcRequest, error: &Error) -> Result<()>;
}

/// Logging middleware
pub struct LoggingMiddleware {
    pub log_requests: bool,
    pub log_responses: bool,
    pub log_timing: bool,
}

impl LoggingMiddleware {
    pub fn new() -> Self {
        Self {
            log_requests: true,
            log_responses: false,
            log_timing: true,
        }
    }

    pub fn with_requests(mut self, enabled: bool) -> Self {
        self.log_requests = enabled;
        self
    }

    pub fn with_responses(mut self, enabled: bool) -> Self {
        self.log_responses = enabled;
        self
    }

    pub fn with_timing(mut self, enabled: bool) -> Self {
        self.log_timing = enabled;
        self
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Middleware for LoggingMiddleware {
    async fn before_request(&self, request: &JsonRpcRequest) -> Result<()> {
        if self.log_requests {
            info!("Request: {} (ID: {:?})", request.method, request.id);
        }
        Ok(())
    }

    async fn after_response(
        &self,
        request: &JsonRpcRequest,
        response: &JsonRpcResponse,
    ) -> Result<()> {
        if self.log_responses {
            if response.error.is_some() {
                warn!(
                    "Response error for {}: {:?}",
                    request.method, response.error
                );
            } else {
                info!(
                    "Response success for {} (ID: {:?})",
                    request.method, request.id
                );
            }
        }
        Ok(())
    }

    async fn on_error(&self, request: &JsonRpcRequest, error: &Error) -> Result<()> {
        error!("Error processing {}: {}", request.method, error);
        Ok(())
    }
}

/// Rate limiting middleware
pub struct RateLimitMiddleware {
    rate_limiter: std::sync::Arc<tokio::sync::Mutex<mocopr_core::utils::RateLimiter>>,
}

impl RateLimitMiddleware {
    pub fn new(max_requests: u32, window_duration: std::time::Duration) -> Self {
        Self {
            rate_limiter: std::sync::Arc::new(tokio::sync::Mutex::new(
                mocopr_core::utils::RateLimiter::new(max_requests, window_duration),
            )),
        }
    }
}

#[async_trait::async_trait]
impl Middleware for RateLimitMiddleware {
    async fn before_request(&self, _request: &JsonRpcRequest) -> Result<()> {
        let mut limiter = self.rate_limiter.lock().await;
        if !limiter.check_rate_limit() {
            return Err(Error::Protocol(
                mocopr_core::error::ProtocolError::RateLimitExceeded,
            ));
        }
        Ok(())
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

/// Authentication middleware
pub struct AuthMiddleware {
    api_keys: std::collections::HashSet<String>,
}

impl AuthMiddleware {
    pub fn new() -> Self {
        Self {
            api_keys: std::collections::HashSet::new(),
        }
    }

    pub fn with_api_key(mut self, key: String) -> Self {
        self.api_keys.insert(key);
        self
    }

    pub fn with_api_keys<I>(mut self, keys: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.api_keys.extend(keys);
        self
    }
}

impl Default for AuthMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Middleware for AuthMiddleware {
    async fn before_request(&self, request: &JsonRpcRequest) -> Result<()> {
        if self.api_keys.is_empty() {
            return Ok(()); // No authentication required
        }

        // Check for API key in request params
        if let Some(params) = &request.params
            && let Some(auth) = params.get("auth")
            && let Some(api_key) = auth.get("api_key")
            && let Some(key_str) = api_key.as_str()
            && self.api_keys.contains(key_str)
        {
            return Ok(());
        }
        Err(Error::Protocol(
            mocopr_core::error::ProtocolError::PermissionDenied,
        ))
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

/// Metrics collection middleware with actual timing measurements
pub struct MetricsMiddleware {
    request_counts: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, u64>>>,
    response_times:
        std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, Vec<u64>>>>,
    // Track start times for in-flight requests
    request_start_times:
        std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, std::time::Instant>>>,
}

impl MetricsMiddleware {
    pub fn new() -> Self {
        Self {
            request_counts: std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            response_times: std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            request_start_times: std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
        }
    }

    pub async fn get_metrics(&self) -> MetricsSnapshot {
        let counts = self.request_counts.read().await;
        let times = self.response_times.read().await;

        MetricsSnapshot {
            request_counts: counts.clone(),
            average_response_times: times
                .iter()
                .map(|(method, times)| {
                    let avg = if times.is_empty() {
                        0.0
                    } else {
                        times.iter().sum::<u64>() as f64 / times.len() as f64
                    };
                    (method.clone(), avg)
                })
                .collect(),
        }
    }
}

impl Default for MetricsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub request_counts: std::collections::HashMap<String, u64>,
    pub average_response_times: std::collections::HashMap<String, f64>,
}

#[async_trait::async_trait]
impl Middleware for MetricsMiddleware {
    async fn before_request(&self, request: &JsonRpcRequest) -> Result<()> {
        // Track request count
        let mut counts = self.request_counts.write().await;
        *counts.entry(request.method.clone()).or_insert(0) += 1;

        // Record request start time - use request ID if available, otherwise method name
        let request_key = request
            .id
            .as_ref()
            .map(|id| match id {
                RequestId::String(s) => s.clone(),
                RequestId::Number(n) => n.to_string(),
            })
            .unwrap_or_else(|| request.method.clone());

        let mut start_times = self.request_start_times.write().await;
        start_times.insert(request_key, std::time::Instant::now());

        Ok(())
    }

    async fn after_response(
        &self,
        request: &JsonRpcRequest,
        _response: &JsonRpcResponse,
    ) -> Result<()> {
        // Calculate actual response time from before_request to after_response
        let request_key = request
            .id
            .as_ref()
            .map(|id| match id {
                RequestId::String(s) => s.clone(),
                RequestId::Number(n) => n.to_string(),
            })
            .unwrap_or_else(|| request.method.clone());

        let mut start_times = self.request_start_times.write().await;
        if let Some(start_time) = start_times.remove(&request_key) {
            let elapsed = start_time.elapsed();
            let elapsed_ms = elapsed.as_millis() as u64;

            let mut times = self.response_times.write().await;
            times
                .entry(request.method.clone())
                .or_insert_with(Vec::new)
                .push(elapsed_ms);
        } else {
            // Fallback: log a warning if we couldn't find the start time
            warn!(
                "No start time found for request: {} (method: {})",
                request_key, request.method
            );
        }

        Ok(())
    }

    async fn on_error(&self, _request: &JsonRpcRequest, _error: &Error) -> Result<()> {
        Ok(())
    }
}
