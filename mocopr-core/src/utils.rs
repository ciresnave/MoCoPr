//! Utility functions and helpers

pub mod json;

use crate::Result;
use serde::{Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Utility functions for MCP implementation
pub struct Utils;

impl Utils {
    /// Get current timestamp as seconds since Unix epoch.
    ///
    /// This utility method provides a consistent way to get the current time
    /// as a Unix timestamp (seconds since January 1, 1970 UTC).
    ///
    /// # Returns
    ///
    /// The number of seconds since the Unix epoch
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::utils::Utils;
    ///
    /// let timestamp = Utils::current_timestamp();
    /// println!("Current timestamp: {}", timestamp);
    /// ```
    pub fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Get current timestamp as milliseconds since Unix epoch.
    ///
    /// This utility method provides a consistent way to get the current time
    /// as a Unix timestamp in milliseconds (thousandths of a second since
    /// January 1, 1970 UTC).
    ///
    /// # Returns
    ///
    /// The number of milliseconds since the Unix epoch
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::utils::Utils;
    ///
    /// let timestamp_millis = Utils::current_timestamp_millis();
    /// println!("Current timestamp (millis): {}", timestamp_millis);
    /// ```
    pub fn current_timestamp_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Validate URI format.
    ///
    /// This utility method checks if a given string is a well-formed URI.
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI string to validate
    ///
    /// # Returns
    ///
    /// `true` if the URI is valid, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::utils::Utils;
    ///
    /// assert!(Utils::validate_uri("https://www.example.com"));
    /// assert!(!Utils::validate_uri("invalid_uri"));
    /// ```
    pub fn validate_uri(uri: &str) -> bool {
        url::Url::parse(uri).is_ok()
    }

    /// Normalize URI by removing trailing slashes and fragments.
    ///
    /// This utility method converts a URI into a canonical form by removing
    /// unnecessary parts like trailing slashes and fragment identifiers.
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI string to normalize
    ///
    /// # Returns
    ///
    /// A normalized URI string
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::utils::Utils;
    ///
    /// let uri = "https://www.example.com/some/path/";
    /// let normalized = Utils::normalize_uri(uri).unwrap();
    /// assert_eq!(normalized, "https://www.example.com/some/path");
    ///
    /// let uri = "https://www.example.com/resource#section1";
    /// let normalized = Utils::normalize_uri(uri).unwrap();
    /// assert_eq!(normalized, "https://www.example.com/resource");
    /// ```
    pub fn normalize_uri(uri: &str) -> Result<String> {
        let mut url = url::Url::parse(uri)?;
        url.set_fragment(None);
        let mut normalized = url.to_string();
        if normalized.ends_with('/') && normalized.len() > 1 {
            normalized.pop();
        }
        Ok(normalized)
    }

    /// Generate a random string.
    ///
    /// This utility method creates a random string of the specified length using
    /// URL-safe base64 encoding.
    ///
    /// # Arguments
    ///
    /// * `length` - The length of the string to generate
    ///
    /// # Returns
    ///
    /// A random string
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::utils::Utils;
    ///
    /// let random_str = Utils::random_string(10);
    /// println!("Random string: {}", random_str);
    /// ```
    pub fn random_string(length: usize) -> String {
        use uuid::Uuid;
        let uuid = Uuid::new_v4().to_string();
        let clean = uuid.replace('-', "");
        if length >= clean.len() {
            clean
        } else {
            clean[..length].to_string()
        }
    }

    /// Format bytes in a human-readable format.
    ///
    /// This utility method converts a byte count into a human-readable string
    /// representation, using appropriate units (B, KB, MB, GB, TB).
    ///
    /// # Arguments
    ///
    /// * `bytes` - The number of bytes to format
    ///
    /// # Returns
    ///
    /// A human-readable string representing the byte count
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::utils::Utils;
    ///
    /// assert_eq!(Utils::format_bytes(1024), "1.00 KB");
    /// assert_eq!(Utils::format_bytes(1536), "1.50 KB");
    /// assert_eq!(Utils::format_bytes(1048576), "1.00 MB");
    /// ```
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        const THRESHOLD: f64 = 1024.0;

        if bytes == 0 {
            return "0 B".to_string();
        }

        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= THRESHOLD && unit_index < UNITS.len() - 1 {
            size /= THRESHOLD;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.2} {}", size, UNITS[unit_index])
        }
    }

    /// Format duration in human readable format.
    ///
    /// This utility method converts a `Duration` value into a human-readable string,
    /// showing the elapsed time in hours, minutes, seconds, and milliseconds.
    ///
    /// # Arguments
    ///
    /// * `duration` - The duration to format
    ///
    /// # Returns
    ///
    /// A human-readable string representing the duration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::utils::Utils;
    ///
    /// let duration = std::time::Duration::new(3661, 500_000_000);
    /// let formatted = Utils::format_duration(duration);
    /// assert_eq!(formatted, "1h 1m 1s");
    /// ```
    pub fn format_duration(duration: std::time::Duration) -> String {
        let total_seconds = duration.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let millis = duration.subsec_millis();

        if hours > 0 {
            format!("{hours}h {minutes}m {seconds}s")
        } else if minutes > 0 {
            format!("{minutes}m {seconds}s")
        } else if seconds > 0 {
            format!("{seconds}.{millis:03}s")
        } else {
            format!("{millis}ms")
        }
    }

    /// Merge two JSON values recursively.
    ///
    /// This utility method merges the contents of one JSON value into another,
    /// recursively combining objects and replacing values as necessary.
    ///
    /// # Arguments
    ///
    /// * `a` - The target JSON value to merge into
    /// * `b` - The source JSON value to merge from
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::utils::Utils;
    ///
    /// let mut a = serde_json::json!({"key1": "value1", "key2": "value2"});
    /// let b = serde_json::json!({"key2": "new_value2", "key3": "value3"});
    ///
    /// Utils::merge_json(&mut a, b);
    ///
    /// assert_eq!(a["key1"], "value1");
    /// assert_eq!(a["key2"], "new_value2");
    /// assert_eq!(a["key3"], "value3");
    /// ```
    pub fn merge_json(a: &mut serde_json::Value, b: serde_json::Value) {
        match (a, b) {
            (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
                for (k, v) in b {
                    Self::merge_json(a.entry(k).or_insert(serde_json::Value::Null), v);
                }
            }
            (a, b) => *a = b,
        }
    }

    /// Sanitize a file path to prevent directory traversal attacks.
    ///
    /// This function removes ".." components and ensures the path doesn't
    /// escape from a base directory when resolved.
    ///
    /// # Security
    ///
    /// This is a critical security function that should be used whenever
    /// accepting file paths from external sources.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::utils::Utils;
    ///
    /// let safe_path = Utils::sanitize_path("../../../etc/passwd");
    /// assert!(!safe_path.to_string_lossy().contains(".."));
    /// ```
    pub fn sanitize_path<P: AsRef<Path>>(path: P) -> PathBuf {
        let path = path.as_ref();
        let mut components = Vec::new();

        for component in path.components() {
            match component {
                std::path::Component::Prefix(_) => {
                    // On Windows, preserve drive prefixes
                    components.push(component);
                }
                std::path::Component::RootDir => {
                    // Preserve root directory
                    components.push(component);
                }
                std::path::Component::CurDir => {
                    // Skip current directory references
                    continue;
                }
                std::path::Component::ParentDir => {
                    // Remove parent directory references to prevent traversal
                    if let Some(last) = components.last()
                        && !matches!(
                            last,
                            std::path::Component::RootDir | std::path::Component::Prefix(_)
                        )
                    {
                        components.pop();
                    }
                }
                std::path::Component::Normal(_name) => {
                    // Keep normal components
                    components.push(component);
                }
            }
        }

        components.iter().collect()
    }

    /// Validate that a URI scheme is allowed.
    ///
    /// Validates that a URI scheme is in the list of allowed schemes.
    ///
    /// This is an important security check to prevent URI-based attacks and
    /// ensure that resources only use approved protocols.
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI to validate
    /// * `allowed_schemes` - List of schemes that are allowed (e.g., `["file", "http", "https"]`)
    ///
    /// # Returns
    ///
    /// `Ok(())` if the scheme is allowed, or an `Error` if not
    ///
    /// # Security
    ///
    /// This method helps prevent protocol-based injection attacks by restricting
    /// URIs to a known set of safe schemes. Always use this validation when
    /// accepting URIs from external sources.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::utils::Utils;
    /// use url::Url;
    ///
    /// let url = Url::parse("https://example.com").unwrap();
    /// assert!(Utils::validate_uri_scheme(&url, &["http", "https"]).is_ok());
    ///
    /// let url = Url::parse("javascript:alert()").unwrap();
    /// assert!(Utils::validate_uri_scheme(&url, &["file", "http", "https"]).is_err());
    /// ```
    pub fn validate_uri_scheme(uri: &url::Url, allowed_schemes: &[&str]) -> Result<()> {
        if allowed_schemes.contains(&uri.scheme()) {
            Ok(())
        } else {
            Err(crate::Error::security(format!(
                "URI scheme '{}' is not allowed. Allowed schemes: {:?}",
                uri.scheme(),
                allowed_schemes
            )))
        }
    }

    /// Validate that a string doesn't contain dangerous characters.
    ///
    /// # Security
    ///
    /// This helps prevent injection attacks by validating input strings.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::utils::Utils;
    ///
    /// assert!(Utils::validate_safe_string("normal_text-123").is_ok());
    /// assert!(Utils::validate_safe_string("text\x00with\x01control").is_err());
    /// ```
    pub fn validate_safe_string(input: &str) -> Result<()> {
        // Check for control characters (except whitespace)
        for ch in input.chars() {
            if ch.is_control() && !ch.is_whitespace() {
                return Err(crate::Error::validation(format!(
                    "String contains unsafe control character: {:?}",
                    ch
                )));
            }
        }

        // Check for null bytes
        if input.contains('\0') {
            return Err(crate::Error::validation(
                "String contains null byte".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate that a file size is within reasonable limits.
    ///
    /// # Security
    ///
    /// This prevents denial of service attacks through extremely large files.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::utils::Utils;
    ///
    /// assert!(Utils::validate_file_size(1024, 1024 * 1024).is_ok()); // 1KB file, 1MB limit
    /// assert!(Utils::validate_file_size(1024 * 1024 * 10, 1024 * 1024).is_err()); // 10MB file, 1MB limit
    /// ```
    pub fn validate_file_size(size: u64, max_size: u64) -> Result<()> {
        if size > max_size {
            Err(crate::Error::validation(format!(
                "File size {} exceeds maximum allowed size {}",
                Self::format_bytes(size),
                Self::format_bytes(max_size)
            )))
        } else {
            Ok(())
        }
    }

    /// Rate limiting check (simple token bucket implementation).
    ///
    /// # Security
    ///
    /// This helps prevent abuse by limiting the rate of operations.
    ///
    /// Note: This is a simple implementation. For production use,
    /// consider using a proper rate limiting library.
    pub fn check_rate_limit(
        last_request: &mut Option<SystemTime>,
        min_interval_ms: u64,
    ) -> Result<()> {
        let now = SystemTime::now();

        if let Some(last) = last_request {
            let elapsed = now
                .duration_since(*last)
                .unwrap_or(std::time::Duration::from_secs(0));

            let min_interval = std::time::Duration::from_millis(min_interval_ms);

            if elapsed < min_interval {
                return Err(crate::Error::validation(format!(
                    "Rate limit exceeded. Please wait {} before making another request.",
                    Self::format_duration(min_interval - elapsed)
                )));
            }
        }

        *last_request = Some(now);
        Ok(())
    }
}

/// Progress tracking utility
#[derive(Debug, Clone)]
pub struct ProgressTracker {
    /// Current progress value
    pub current: f64,
    /// Total progress value
    pub total: f64,
    /// Start time of the progress tracking
    pub start_time: std::time::Instant,
}

impl ProgressTracker {
    /// Creates a new progress tracker
    ///
    /// # Arguments
    /// * `total` - Total progress value
    pub fn new(total: f64) -> Self {
        Self {
            current: 0.0,
            total,
            start_time: std::time::Instant::now(),
        }
    }

    /// Updates the current progress
    ///
    /// # Arguments
    /// * `current` - New current progress value
    pub fn update(&mut self, current: f64) {
        self.current = current.min(self.total);
    }

    /// Increments the current progress
    ///
    /// # Arguments
    /// * `amount` - Amount to increment by
    pub fn increment(&mut self, amount: f64) {
        self.current = (self.current + amount).min(self.total);
    }

    /// Gets the progress percentage
    ///
    /// # Returns
    /// Progress percentage (0.0 to 100.0)
    pub fn percentage(&self) -> f64 {
        if self.total == 0.0 {
            0.0
        } else {
            (self.current / self.total * 100.0).min(100.0)
        }
    }

    /// Checks if progress is complete
    ///
    /// # Returns
    /// True if progress is complete
    pub fn is_complete(&self) -> bool {
        self.current >= self.total
    }

    /// Gets the elapsed time since start
    ///
    /// # Returns
    /// Elapsed duration
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Estimates remaining time
    ///
    /// # Returns
    /// Estimated remaining duration, or None if cannot estimate
    pub fn estimated_remaining(&self) -> Option<std::time::Duration> {
        if self.current == 0.0 || self.is_complete() {
            return None;
        }

        let elapsed = self.elapsed();
        let rate = self.current / elapsed.as_secs_f64();
        let remaining = (self.total - self.current) / rate;

        Some(std::time::Duration::from_secs_f64(remaining))
    }
}

/// Rate limiter utility
#[derive(Debug)]
pub struct RateLimiter {
    max_requests: u32,
    window_duration: std::time::Duration,
    requests: std::collections::VecDeque<std::time::Instant>,
}

impl RateLimiter {
    /// Creates a new rate limiter
    ///
    /// # Arguments
    /// * `max_requests` - Maximum number of requests allowed
    /// * `window_duration` - Time window for rate limiting
    pub fn new(max_requests: u32, window_duration: std::time::Duration) -> Self {
        Self {
            max_requests,
            window_duration,
            requests: std::collections::VecDeque::new(),
        }
    }

    /// Checks if a request can be made within rate limits
    ///
    /// # Returns
    /// True if request is allowed, false if rate limited
    pub fn check_rate_limit(&mut self) -> bool {
        let now = std::time::Instant::now();
        let cutoff = now - self.window_duration;

        // Remove old requests
        while let Some(&front) = self.requests.front() {
            if front < cutoff {
                self.requests.pop_front();
            } else {
                break;
            }
        }

        // Check if we can make another request
        if self.requests.len() < self.max_requests as usize {
            self.requests.push_back(now);
            true
        } else {
            false
        }
    }

    /// Gets the number of remaining requests
    ///
    /// # Returns
    /// Number of remaining requests within the current window
    pub fn remaining(&self) -> u32 {
        self.max_requests.saturating_sub(self.requests.len() as u32)
    }

    /// Gets the time when the rate limit will reset
    ///
    /// # Returns
    /// Instant when rate limit resets, or None if no requests made
    pub fn reset_time(&self) -> Option<std::time::Instant> {
        self.requests
            .front()
            .map(|&first| first + self.window_duration)
    }
}
