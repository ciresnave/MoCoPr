//! Context extraction for conditional permissions

use crate::error::RbacError;
use mocopr_core::prelude::*;

use async_trait::async_trait;
use chrono::{Timelike, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;

// Use fully qualified Result to avoid ambiguity
type RbacResult<T> = std::result::Result<T, RbacError>;

/// Configuration for trust level assignment based on IP ranges
#[derive(Debug, Clone)]
pub struct TrustLevelConfig {
    /// IP ranges mapped to trust levels
    pub ip_ranges: HashMap<String, String>,
    /// Default trust level for unknown IPs
    pub default_trust_level: String,
    /// Whether to enable strict IP checking
    pub strict_mode: bool,
}

impl Default for TrustLevelConfig {
    fn default() -> Self {
        let mut ip_ranges = HashMap::new();
        // Example configuration - replace with your actual IP ranges
        ip_ranges.insert("192.168.0.0/16".to_string(), "high".to_string());
        ip_ranges.insert("10.0.0.0/8".to_string(), "high".to_string());
        ip_ranges.insert("172.16.0.0/12".to_string(), "medium".to_string());

        Self {
            ip_ranges,
            default_trust_level: "low".to_string(),
            strict_mode: false,
        }
    }
}

impl TrustLevelConfig {
    /// Get trust level for the given IP address
    pub fn get_trust_level(&self, ip: &str) -> Option<String> {
        // Parse IP address
        if let Ok(addr) = IpAddr::from_str(ip) {
            // Check each configured range
            for (cidr_range, trust_level) in &self.ip_ranges {
                if self.ip_in_range(&addr, cidr_range) {
                    return Some(trust_level.clone());
                }
            }
        }

        // Return default trust level if no match found
        Some(self.default_trust_level.clone())
    }

    /// Check if IP is in the given CIDR range (basic implementation)
    /// For production use, consider using a proper CIDR library like `cidr` or `ipnet`
    fn ip_in_range(&self, ip: &IpAddr, cidr: &str) -> bool {
        // Basic CIDR matching - in production, use a proper CIDR library
        if let Some((network, prefix)) = cidr.split_once('/') {
            if let (Ok(network_ip), Ok(prefix_len)) =
                (IpAddr::from_str(network), prefix.parse::<u8>())
            {
                match (ip, network_ip) {
                    (IpAddr::V4(ip), IpAddr::V4(net)) => {
                        let ip_bits = u32::from(*ip);
                        let net_bits = u32::from(net);
                        let mask = (!0u32) << (32 - prefix_len);
                        (ip_bits & mask) == (net_bits & mask)
                    }
                    // IPv6 support would go here
                    _ => false,
                }
            } else {
                false
            }
        } else {
            // Exact IP match
            if let Ok(exact_ip) = IpAddr::from_str(cidr) {
                *ip == exact_ip
            } else {
                false
            }
        }
    }
}

/// Trait for extracting context from MCP requests
#[async_trait]
pub trait ContextExtractor {
    /// Extract context information from a request
    async fn extract_context(
        &self,
        request: &JsonRpcRequest,
    ) -> RbacResult<HashMap<String, String>>;
}

/// Default context extractor
pub struct DefaultContextExtractor;

#[async_trait]
impl ContextExtractor for DefaultContextExtractor {
    async fn extract_context(
        &self,
        request: &JsonRpcRequest,
    ) -> RbacResult<HashMap<String, String>> {
        let mut context = HashMap::new();

        // Add timestamp
        let now = Utc::now();
        context.insert("timestamp".to_string(), now.to_rfc3339());
        context.insert("date".to_string(), now.format("%Y-%m-%d").to_string());
        context.insert("time".to_string(), now.format("%H:%M:%S").to_string());

        // Add business hours flag
        let hour = now.hour();
        let is_business_hours = (9..=17).contains(&hour); // 9 AM to 5 PM
        context.insert("business_hours".to_string(), is_business_hours.to_string());

        // Add day of week
        context.insert("day_of_week".to_string(), now.format("%A").to_string());
        context.insert(
            "is_weekend".to_string(),
            (hour == 6 || hour == 0).to_string(),
        ); // Sunday = 0, Saturday = 6

        // Extract any context from request parameters
        if let Some(params) = &request.params {
            if let Some(context_obj) = params.get("context") {
                self.extract_from_json_value(context_obj, &mut context)?;
            }

            // Extract auth context
            if let Some(auth) = params.get("auth") {
                if let Some(user_id) = auth.get("user_id")
                    && let Some(id) = user_id.as_str()
                {
                    context.insert("user_id".to_string(), id.to_string());
                }
                if let Some(session_id) = auth.get("session_id")
                    && let Some(id) = session_id.as_str()
                {
                    context.insert("session_id".to_string(), id.to_string());
                }
                if let Some(client_ip) = auth.get("client_ip")
                    && let Some(ip) = client_ip.as_str()
                {
                    context.insert("client_ip".to_string(), ip.to_string());
                }
            }
        }

        // Add request method as context
        context.insert("method".to_string(), request.method.clone());

        Ok(context)
    }
}

impl DefaultContextExtractor {
    fn extract_from_json_value(
        &self,
        value: &Value,
        context: &mut HashMap<String, String>,
    ) -> RbacResult<()> {
        match value {
            Value::Object(obj) => {
                for (key, val) in obj {
                    match val {
                        Value::String(s) => {
                            context.insert(key.clone(), s.clone());
                        }
                        Value::Number(n) => {
                            context.insert(key.clone(), n.to_string());
                        }
                        Value::Bool(b) => {
                            context.insert(key.clone(), b.to_string());
                        }
                        _ => {
                            // For complex types, convert to JSON string
                            context.insert(key.clone(), val.to_string());
                        }
                    }
                }
            }
            _ => {
                return Err(RbacError::ContextExtraction(
                    "Context must be a JSON object".to_string(),
                ));
            }
        }
        Ok(())
    }
}

// Type alias for context extractor function
type ContextExtractorFn = Box<dyn Fn(&JsonRpcRequest) -> Option<String> + Send + Sync>;

/// Extended context extractor with additional features
pub struct ExtendedContextExtractor {
    /// Custom extractors for specific context keys
    custom_extractors: HashMap<String, ContextExtractorFn>,
}

impl ExtendedContextExtractor {
    pub fn new() -> Self {
        Self {
            custom_extractors: HashMap::new(),
        }
    }

    /// Add a custom context extractor
    pub fn with_custom_extractor<F>(mut self, key: &str, extractor: F) -> Self
    where
        F: Fn(&JsonRpcRequest) -> Option<String> + Send + Sync + 'static,
    {
        self.custom_extractors
            .insert(key.to_string(), Box::new(extractor));
        self
    }

    /// Add extractor for client trust level based on configurable IP ranges
    ///
    /// This now supports configurable IP allowlists for production use.
    /// Pass your own trust level configuration instead of using hardcoded ranges.
    pub fn with_trust_level_extractor(self) -> Self {
        self.with_configurable_trust_level_extractor(TrustLevelConfig::default())
    }

    /// Add configurable trust level extractor with custom IP ranges
    pub fn with_configurable_trust_level_extractor(self, config: TrustLevelConfig) -> Self {
        self.with_custom_extractor("trust_level", move |request| {
            if let Some(params) = &request.params
                && let Some(auth) = params.get("auth")
                && let Some(client_ip) = auth.get("client_ip")
                && let Some(ip) = client_ip.as_str()
            {
                return config.get_trust_level(ip);
            }
            Some(config.default_trust_level.clone())
        })
    }

    /// Add extractor for geographic location
    ///
    /// This implementation provides basic IP-based location mapping.
    /// For production use, integrate with a GeoIP service like MaxMind GeoLite2.
    ///
    /// # Example with GeoIP integration
    ///
    /// ```ignore
    /// // Production implementation would look like:
    /// use maxminddb::Reader;
    ///
    /// let reader = Reader::open_readfile("GeoLite2-City.mmdb")?;
    /// pub fn with_geoip_location_extractor(self, reader: Reader<Vec<u8>>) -> Self {
    ///     self.with_custom_extractor("location", move |request| {
    ///         if let Some(client_ip) = extract_client_ip(request) {
    ///             if let Ok(ip) = client_ip.parse::<std::net::IpAddr>() {
    ///                 if let Ok(result) = reader.lookup::<maxminddb::geoip2::City>(ip) {
    ///                     if let Some(country) = result.country {
    ///                         if let Some(name) = country.names {
    ///                             return name.get("en").map(|s| s.to_string());
    ///                         }
    ///                     }
    ///                 }
    ///             }
    ///         }
    ///         Some("unknown".to_string())
    ///     })
    /// }
    /// ```
    pub fn with_location_extractor(self) -> Self {
        self.with_custom_extractor("location", |request| {
            // Basic IP-to-region mapping for common IP ranges
            // This is a simplified implementation for demonstration
            if let Some(client_ip) = extract_client_ip_from_auth(request) {
                // Very basic regional classification based on IP ranges
                // This is NOT production-ready and should use a real GeoIP service
                if client_ip.starts_with("192.168.")
                    || client_ip.starts_with("10.")
                    || client_ip.starts_with("127.")
                {
                    return Some("local".to_string());
                } else if client_ip.starts_with("8.8.") || client_ip.starts_with("1.1.") {
                    return Some("public-dns".to_string());
                } else {
                    // For production, replace this with actual GeoIP lookup
                    return Some("external".to_string());
                }
            }

            Some("unknown".to_string())
        })
    }
}

#[async_trait]
impl ContextExtractor for ExtendedContextExtractor {
    async fn extract_context(
        &self,
        request: &JsonRpcRequest,
    ) -> RbacResult<HashMap<String, String>> {
        // Start with default context
        let mut context = DefaultContextExtractor
            .extract_context(request)
            .await
            .map_err(|e| RbacError::ContextExtraction(e.to_string()))?;

        // Apply custom extractors
        for (key, extractor) in &self.custom_extractors {
            if let Some(value) = extractor(request) {
                context.insert(key.clone(), value);
            }
        }

        Ok(context)
    }
}

impl Default for ExtendedContextExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for common conditional permission patterns
pub struct ContextConditions;

impl ContextConditions {
    /// Check if current time is during business hours
    pub fn business_hours_only() -> impl Fn(&HashMap<String, String>) -> bool + Send + Sync + 'static
    {
        |context| {
            context
                .get("business_hours")
                .map(|v| v == "true")
                .unwrap_or(false)
        }
    }

    /// Check if request is from high trust level client
    pub fn high_trust_only() -> impl Fn(&HashMap<String, String>) -> bool + Send + Sync + 'static {
        |context| {
            context
                .get("trust_level")
                .map(|v| v == "high")
                .unwrap_or(false)
        }
    }

    /// Check if request is during weekdays
    pub fn weekdays_only() -> impl Fn(&HashMap<String, String>) -> bool + Send + Sync + 'static {
        |context| {
            context
                .get("is_weekend")
                .map(|v| v == "false")
                .unwrap_or(true)
        }
    }

    /// Check if user is specified user
    pub fn user_only(
        user_id: String,
    ) -> impl Fn(&HashMap<String, String>) -> bool + Send + Sync + 'static {
        move |context| {
            context
                .get("user_id")
                .map(|v| v == &user_id)
                .unwrap_or(false)
        }
    }

    /// Combine multiple conditions with AND logic
    pub fn all_of<F>(
        conditions: Vec<F>,
    ) -> impl Fn(&HashMap<String, String>) -> bool + Send + Sync + 'static
    where
        F: Fn(&HashMap<String, String>) -> bool + Send + Sync + 'static,
    {
        move |context| conditions.iter().all(|condition| condition(context))
    }

    /// Combine multiple conditions with OR logic
    pub fn any_of<F>(
        conditions: Vec<F>,
    ) -> impl Fn(&HashMap<String, String>) -> bool + Send + Sync + 'static
    where
        F: Fn(&HashMap<String, String>) -> bool + Send + Sync + 'static,
    {
        move |context| conditions.iter().any(|condition| condition(context))
    }
}

/// Helper function to extract client IP from request authentication data
///
/// This looks for client_ip in the request's authentication headers or metadata.
/// For HTTP transports, this would typically be extracted from headers like
/// X-Forwarded-For, X-Real-IP, or from the connection metadata.
fn extract_client_ip_from_auth(request: &mocopr_core::JsonRpcRequest) -> Option<String> {
    // Check if there's authentication data in the request
    if let Some(auth_data) = request
        .params
        .as_ref()
        .and_then(|params| params.get("auth"))
        .and_then(|auth| auth.as_object())
        && let Some(client_ip) = auth_data.get("client_ip").and_then(|ip| ip.as_str())
    {
        return Some(client_ip.to_string());
    }

    // For production, you'd also check HTTP headers like:
    // - X-Forwarded-For
    // - X-Real-IP
    // - CF-Connecting-IP (Cloudflare)
    // - True-Client-IP

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_default_context_extractor() {
        let extractor = DefaultContextExtractor;
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(RequestId::Number(1)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "auth": {
                    "user_id": "test_user",
                    "client_ip": "192.168.1.100"
                },
                "context": {
                    "custom_field": "custom_value"
                }
            })),
        };

        let context = extractor.extract_context(&request).await.unwrap();

        assert_eq!(context.get("method").unwrap(), "tools/call");
        assert_eq!(context.get("user_id").unwrap(), "test_user");
        assert_eq!(context.get("client_ip").unwrap(), "192.168.1.100");
        assert_eq!(context.get("custom_field").unwrap(), "custom_value");
        assert!(context.contains_key("timestamp"));
        assert!(context.contains_key("business_hours"));
    }

    #[test]
    fn test_context_conditions() {
        let mut context = HashMap::new();
        context.insert("business_hours".to_string(), "true".to_string());
        context.insert("trust_level".to_string(), "high".to_string());
        context.insert("user_id".to_string(), "admin".to_string());

        assert!(ContextConditions::business_hours_only()(&context));
        assert!(ContextConditions::high_trust_only()(&context));
        assert!(ContextConditions::user_only("admin".to_string())(&context));
        assert!(!ContextConditions::user_only("other".to_string())(&context));
    }
}
