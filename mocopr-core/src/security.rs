// Security validation and hardening implementation
// This addresses the failing integration test by implementing actual security checks

use crate::Error;
use crate::utils::Utils;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::warn;
use url::Url;

/// Comprehensive security validator for MCP operations
pub struct SecurityValidator {
    /// Allowed URI schemes
    pub allowed_schemes: Vec<String>,
    /// Maximum file size for operations
    pub max_file_size: u64,
    /// Allowed file extensions
    pub allowed_extensions: Vec<String>,
    /// Root directory for file operations
    pub root_directory: Option<PathBuf>,
}

impl Default for SecurityValidator {
    fn default() -> Self {
        Self {
            allowed_schemes: vec!["file".to_string(), "http".to_string(), "https".to_string()],
            max_file_size: 10 * 1024 * 1024, // 10MB
            allowed_extensions: vec![
                "txt".to_string(),
                "md".to_string(),
                "json".to_string(),
                "yml".to_string(),
                "yaml".to_string(),
                "xml".to_string(),
                "csv".to_string(),
                "log".to_string(),
            ],
            root_directory: None,
        }
    }
}

impl SecurityValidator {
    /// Create a new security validator with custom settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set allowed URI schemes
    pub fn with_allowed_schemes(mut self, schemes: Vec<String>) -> Self {
        self.allowed_schemes = schemes;
        self
    }

    /// Set maximum file size
    pub fn with_max_file_size(mut self, size: u64) -> Self {
        self.max_file_size = size;
        self
    }

    /// Set allowed file extensions
    pub fn with_allowed_extensions(mut self, extensions: Vec<String>) -> Self {
        self.allowed_extensions = extensions;
        self
    }

    /// Set root directory for file operations
    pub fn with_root_directory(mut self, root: PathBuf) -> Self {
        self.root_directory = Some(root);
        self
    }

    /// Validate a URI for security compliance
    pub fn validate_uri(&self, uri: &Url) -> Result<()> {
        // Check scheme
        if !self.allowed_schemes.contains(&uri.scheme().to_string()) {
            return Err(Error::security(format!(
                "URI scheme '{}' is not allowed. Allowed schemes: {:?}",
                uri.scheme(),
                self.allowed_schemes
            ))
            .into());
        }

        // Additional validation for file URIs
        if uri.scheme() == "file"
            && let Ok(path) = uri.to_file_path()
        {
            self.validate_file_path(&path)?;
        }

        Ok(())
    }

    /// Validate a file path for security compliance
    pub fn validate_file_path(&self, path: &Path) -> Result<()> {
        // Sanitize path to prevent directory traversal
        let sanitized = Utils::sanitize_path(path);

        // Check if path is within allowed root directory
        if let Some(root) = &self.root_directory {
            let canonical_root = fs::canonicalize(root).map_err(|e| {
                Error::security(format!("Failed to canonicalize root directory: {}", e))
            })?;

            let canonical_path = fs::canonicalize(&sanitized)
                .map_err(|e| Error::security(format!("Failed to canonicalize file path: {}", e)))?;

            if !canonical_path.starts_with(&canonical_root) {
                return Err(Error::security(format!(
                    "Path '{}' is outside of allowed directory '{}'",
                    canonical_path.display(),
                    canonical_root.display()
                ))
                .into());
            }
        }

        // Check file extension
        if let Some(extension) = path.extension() {
            let ext_str = extension.to_string_lossy().to_lowercase();
            if !self.allowed_extensions.contains(&ext_str) {
                warn!(
                    "File extension '{}' is not in allowed list: {:?}",
                    ext_str, self.allowed_extensions
                );
                return Err(Error::security(format!(
                    "File extension '{}' is not allowed. Allowed extensions: {:?}",
                    ext_str, self.allowed_extensions
                ))
                .into());
            }
        }

        Ok(())
    }

    /// Validate a file path from string for security compliance
    pub fn validate_file_path_str(&self, path: &str) -> Result<()> {
        self.validate_file_path(Path::new(path))
    }

    /// Validate a PathBuf for security compliance
    pub fn validate_file_path_buf(&self, path: &Path) -> Result<()> {
        self.validate_file_path(path)
    }

    /// Validate file size
    pub fn validate_file_size(&self, size: u64) -> Result<()> {
        Ok(Utils::validate_file_size(size, self.max_file_size)?)
    }

    /// Validate string input for safety
    pub fn validate_string_input(&self, input: &str) -> Result<()> {
        Ok(Utils::validate_safe_string(input)?)
    }

    /// Comprehensive resource validation
    pub fn validate_resource_access(&self, uri: &Url) -> Result<()> {
        // Basic URI validation
        self.validate_uri(uri)?;

        // For file URIs, perform additional checks
        if uri.scheme() == "file"
            && let Ok(path) = uri.to_file_path()
        {
            // Check if file exists
            if !path.exists() {
                return Err(
                    Error::not_found(format!("File does not exist: {}", path.display())).into(),
                );
            }

            // Check file size
            if let Ok(metadata) = fs::metadata(&path) {
                self.validate_file_size(metadata.len())?;
            } else {
                return Err(Error::security(format!(
                    "Cannot read file metadata: {}",
                    path.display()
                ))
                .into());
            }
        }
        Ok(())
    }

    /// Validate tool parameters
    pub fn validate_tool_parameters(&self, params: &serde_json::Value) -> Result<()> {
        // Recursively validate all string values in the parameter object
        match params {
            serde_json::Value::String(s) => {
                self.validate_string_input(s)?;
            }
            serde_json::Value::Object(obj) => {
                for (key, value) in obj {
                    self.validate_string_input(key)?;
                    self.validate_tool_parameters(value)?;
                }
            }
            serde_json::Value::Array(arr) => {
                for value in arr {
                    self.validate_tool_parameters(value)?;
                }
            }
            _ => {} // Numbers, booleans, null are safe
        }

        Ok(())
    }
}

/// Error recovery and resilience system
pub struct ErrorRecoverySystem {
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Whether to log errors
    pub log_errors: bool,
}

impl Default for ErrorRecoverySystem {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 1000,
            log_errors: true,
        }
    }
}

impl ErrorRecoverySystem {
    /// Create a new error recovery system
    pub fn new() -> Self {
        Self::default()
    }

    /// Execute an operation with retry logic
    pub async fn execute_with_retry<F, T, E>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Result<T, E> + Send + Sync,
        E: std::error::Error + Send + Sync + 'static,
        T: Send + Sync,
    {
        let mut attempts = 0;

        loop {
            match operation() {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempts += 1;

                    if self.log_errors {
                        warn!(
                            "Operation failed (attempt {}/{}): {}",
                            attempts, self.max_retries, e
                        );
                    }

                    if attempts >= self.max_retries {
                        return Err(Error::operation_failed(format!(
                            "Operation failed after {} attempts: {}",
                            self.max_retries, e
                        ))
                        .into());
                    }

                    // Wait before retry
                    tokio::time::sleep(tokio::time::Duration::from_millis(self.retry_delay_ms))
                        .await;
                }
            }
        }
    }

    /// Handle invalid method calls gracefully
    pub fn handle_invalid_method(&self, method: &str) -> Error {
        if self.log_errors {
            warn!("Invalid method called: {}", method);
        }

        Error::method_not_found(format!(
            "Method '{}' is not supported. Available methods should be checked through capability negotiation.",
            method
        ))
    }

    /// Handle invalid parameters gracefully
    pub fn handle_invalid_parameters(&self, method: &str, error: &str) -> Error {
        if self.log_errors {
            warn!("Invalid parameters for method '{}': {}", method, error);
        }

        Error::invalid_params(format!(
            "Invalid parameters for method '{}': {}. Please check the method signature and required parameters.",
            method, error
        ))
    }

    /// Handle resource access errors gracefully
    pub fn handle_resource_error(&self, uri: &str, error: &str) -> Error {
        if self.log_errors {
            warn!("Resource access error for '{}': {}", uri, error);
        }

        Error::resource_error(format!(
            "Failed to access resource '{}': {}. Please check the resource exists and is accessible.",
            uri, error
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_security_validator_path_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let validator = SecurityValidator::new().with_root_directory(temp_dir.path().to_path_buf());

        // Create a test file within the allowed directory
        let allowed_file = temp_dir.path().join("allowed.txt");
        fs::write(&allowed_file, "test content").unwrap();

        // Test allowed access
        assert!(validator.validate_file_path(&allowed_file).is_ok());

        // Test path traversal attempts
        let traversal_attempts = vec![
            temp_dir.path().join("../../../etc/passwd"),
            temp_dir.path().join("../../sensitive.txt"),
            temp_dir.path().join("../outside/file.txt"),
        ];

        for malicious_path in traversal_attempts {
            assert!(validator.validate_file_path(&malicious_path).is_err());
        }
    }

    #[test]
    fn test_security_validator_file_extensions() {
        let validator = SecurityValidator::new()
            .with_allowed_extensions(vec!["txt".to_string(), "md".to_string()]);

        // Test allowed extensions
        assert!(validator.validate_file_path(Path::new("test.txt")).is_ok());
        assert!(validator.validate_file_path(Path::new("test.md")).is_ok());

        // Test disallowed extensions
        assert!(validator.validate_file_path(Path::new("test.exe")).is_err());
        assert!(validator.validate_file_path(Path::new("test.sh")).is_err());
    }

    #[tokio::test]
    async fn test_error_recovery_system() {
        let recovery = ErrorRecoverySystem::new();

        // Test successful operation
        let result = recovery
            .execute_with_retry(|| -> Result<i32, std::io::Error> { Ok(42) })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        // Test operation that fails then succeeds
        let attempts = std::sync::Arc::new(std::sync::Mutex::new(0));
        let attempts_clone = attempts.clone();
        let result = recovery
            .execute_with_retry(move || -> Result<i32, std::io::Error> {
                let mut attempts_ref = attempts_clone.lock().unwrap();
                *attempts_ref += 1;
                if *attempts_ref < 3 {
                    Err(std::io::Error::other("temporary failure"))
                } else {
                    Ok(42)
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}
