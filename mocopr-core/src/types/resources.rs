//! Resource-related types and messages for the Model Context Protocol.
//!
//! This module defines all the types and structures needed to work with MCP resources.
//! Resources represent any piece of data or content that an MCP server can provide,
//! such as files, database records, API responses, or any other structured data.
//!
//! Resources are identified by URIs and can be read, listed, and subscribed to for updates.
//!
//! # Example
//!
//! ```rust
//! use mocopr_core::types::Resource;
//! use url::Url;
//!
//! let resource = Resource {
//!     uri: Url::parse("file:///path/to/file.txt").unwrap(),
//!     name: "file.txt".to_string(),
//!     description: Some("A text file containing important data".to_string()),
//!     mime_type: Some("text/plain".to_string()),
//!     annotations: None,
//! };
//! ```

use super::*;

/// Resource represents any piece of data or content that an MCP server provides.
///
/// Resources are identified by URIs and can contain any type of data including
/// files, database records, API responses, or other structured content. They
/// support MIME types for content identification and annotations for metadata.
///
/// # MCP Specification Compliance
///
/// Resources follow the MCP specification's resource model, where:
/// - Each resource has a unique URI that identifies it
/// - Resources have human-readable names and optional descriptions
/// - MIME types indicate the content format
/// - Custom annotations can provide additional metadata
///
/// # Builder Pattern
///
/// The `Resource` struct implements a builder pattern for constructing instances:
///
/// ```rust
/// use mocopr_core::types::Resource;
/// use url::Url;
/// use serde_json::json;
///
/// let uri = Url::parse("file:///path/to/data.json").unwrap();
/// let resource = Resource::new(uri, "Config File")
///     .with_description("Application configuration")
///     .with_mime_type("application/json")
///     .with_annotations(json!({
///         "version": "1.0.0",
///         "author": "MoCoPr Team",
///     }));
/// ```
//         "last_updated": "2025-06-18"
//     }));
// ```
//
// # Security Considerations
//
// When creating resources from external inputs, use `new_validated()` with an explicit
// allowed schemes list to prevent URI scheme-based injection attacks.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Resource {
    /// Unique URI identifying this resource.
    ///
    /// The URI scheme (e.g., "file:", "http:") determines how the resource is accessed.
    /// Common schemes include:
    /// - `file:` for local file system resources
    /// - `http:` and `https:` for web-based resources
    /// - `memory:` for in-memory resources
    /// - Custom schemes for application-specific resource types
    pub uri: Url,

    /// Human-readable name for this resource.
    ///
    /// This name is used for display purposes and should be concise but descriptive.
    /// It does not need to be unique, but should help users identify the resource.
    pub name: String,

    /// Optional description of what this resource contains.
    ///
    /// Provides more detailed information about the resource's purpose or contents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// MIME type of the resource content (e.g., "text/plain", "application/json").
    ///
    /// This helps clients understand how to interpret the resource data.
    /// Common MIME types include:
    /// - `text/plain` for plain text
    /// - `application/json` for JSON data
    /// - `application/octet-stream` for binary data
    #[serde(rename = "mimeType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Optional annotations providing additional metadata about the resource.
    ///
    /// Annotations can contain arbitrary JSON data to extend the resource
    /// with application-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<serde_json::Value>,
}

/// Resource content when reading a resource.
///
/// This structure contains the actual content of a resource when it's read,
/// along with metadata such as the URI and MIME type.
///
/// # Content Types
///
/// The `contents` field can contain multiple content pieces, which may be:
/// - Text content as Unicode strings
/// - Binary content as base64-encoded data
///
/// # MCP Specification Compliance
///
/// The `ResourceContent` type is used in the response to resource read operations
/// in the MCP protocol. It follows the specification's data model for resource content.
///
/// # Examples
///
/// Text content:
/// ```rust
/// use mocopr_core::types::{ResourceContent, Content, TextContent};
/// use url::Url;
///
/// let content = ResourceContent {
///     uri: Url::parse("file:///data.json").unwrap(),
///     mime_type: Some("application/json".to_string()),
///     contents: vec![
///         Content::Text(TextContent::new(r#"{"key": "value"}"#))
///     ],
/// };
/// ```
///
/// Binary content:
/// ```rust
/// use mocopr_core::types::{ResourceContent, Content, ImageContent};
/// use url::Url;
///
/// let content = ResourceContent {
///     uri: Url::parse("file:///image.png").unwrap(),
///     mime_type: Some("image/png".to_string()),
///     contents: vec![
///         Content::Image(ImageContent::new("base64encodeddata", "image/png"))
///     ],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    /// URI of the resource that was read.
    ///
    /// This should match the URI used in the read resource request.
    /// It uniquely identifies the resource within the MCP server's scope.
    pub uri: Url,

    /// MIME type of the content.
    ///
    /// Indicates the format of the data (e.g., "text/plain", "application/json", "image/png").
    /// This helps clients correctly interpret and process the resource content.
    #[serde(rename = "mimeType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// The actual content data in various formats.
    ///
    /// A resource may contain multiple content pieces, each represented as either
    /// Text or Binary content. For example, a document might include both text
    /// and embedded images as separate content entries.
    pub contents: Vec<Content>,
}

/// Alias for ResourceContent for backward compatibility.
///
/// Use ResourceContent for new code, as this alias may be deprecated in future versions.
pub type ResourceContents = ResourceContent;

/// Request to list available resources.
///
/// This message is used by clients to request a list of available resources from an MCP server.
/// It supports pagination to handle large resource collections efficiently.
///
/// # MCP Specification Compliance
///
/// This request corresponds to the `resources/list` method in the MCP specification.
///
/// # Fields
///
/// * `pagination` - Parameters to limit the number of results returned and specify the starting offset
///
/// # Example
///
/// ```rust
/// use mocopr_core::types::{ResourcesListRequest, PaginationParams};
///
/// let request = ResourcesListRequest {
///     pagination: PaginationParams {
///         cursor: Some("next_page_token".to_string()),
///     },
/// };
/// ```
///
/// # JSON Representation
///
/// ```json
/// {
///   "limit": 10,
///   "offset": 20
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesListRequest {
    /// Pagination parameters to limit and offset results
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// Response to list resources request
///
/// This message is returned by an MCP server in response to a `ResourcesListRequest`.
/// It contains the list of available resources along with pagination information.
///
/// # MCP Specification Compliance
///
/// Response returned by the `resources/list` method in the MCP specification.
///
/// This struct encapsulates a list of available resources along with pagination information
/// when the list is too large to return at once. The `next_cursor` field can be used in
/// subsequent requests to retrieve the next page of results.
///
/// # Fields
///
/// * `resources` - A list of resource objects available to the client
/// * `next_cursor` - An optional pagination token used to retrieve the next set of results
/// * `meta` - Additional metadata associated with the response
///
/// # Example
///
/// ```rust
/// use mocopr_core::types::{ResourcesListResponse, Resource, ResponseMetadata};
/// use url::Url;
///
/// let response = ResourcesListResponse {
///     resources: vec![
///         Resource::new(
///             Url::parse("file:///document.txt").unwrap(),
///             "Document"
///         ).with_mime_type("text/plain"),
///         Resource::new(
///             Url::parse("file:///config.json").unwrap(),
///             "Configuration"
///         ).with_mime_type("application/json"),
///     ],
///     next_cursor: Some("next-page-token".to_string()),
///     meta: ResponseMetadata::default(),
/// };
/// ```
///
/// # JSON Representation
///
/// ```json
/// {
///   "resources": [
///     {
///       "uri": "file:///document.txt",
///       "title": "Document",
///       "mimeType": "text/plain"
///     },
///     {
///       "uri": "file:///config.json",
///       "title": "Configuration",
///       "mimeType": "application/json"
///     }
///   ],
///   "nextCursor": "next-page-token"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesListResponse {
    /// The list of available resources
    pub resources: Vec<Resource>,

    /// Optional pagination token for retrieving the next set of results
    #[serde(rename = "nextCursor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,

    /// Additional metadata associated with the response
    #[serde(flatten)]
    pub meta: ResponseMetadata,
}

/// Request to read the content of a specific resource identified by its URI.
///
/// This request corresponds to the `resources/read` method in the MCP specification.
/// It retrieves the content of a resource, which can be text, binary data, or other formats
/// depending on the resource type.
///
/// # Fields
///
/// * `uri` - The unique identifier (URI) of the resource to be read
///
/// # Example
///
/// ```rust
/// use mocopr_core::types::ResourcesReadRequest;
/// use url::Url;
///
/// let request = ResourcesReadRequest {
///     uri: Url::parse("file:///document.txt").unwrap(),
/// };
/// ```
///
/// # JSON Representation
///
/// ```json
/// {
///   "uri": "file:///document.txt"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesReadRequest {
    /// The unique identifier (URI) of the resource to be read
    pub uri: Url,
}

/// Response returned by the `resources/read` method containing the contents of a requested resource.
///
/// This response includes the contents of the requested resource, which may be provided
/// in multiple formats or chunks (hence the vector). The content can be text, binary data,
/// or other formats as specified by the resource's MIME type.
///
/// # Fields
///
/// * `contents` - A vector of resource contents, which may include different representations
///   or chunks of the same resource
/// * `meta` - Additional metadata associated with the response
///
/// # Example
///
/// ```rust
/// use mocopr_core::types::{ResourcesReadResponse, ResourceContent, Content, TextContent, ResponseMetadata};
/// use url::Url;
///
/// let response = ResourcesReadResponse {
///     contents: vec![
///         ResourceContent {
///             uri: Url::parse("file:///document.txt").unwrap(),
///             mime_type: Some("text/plain".to_string()),
///             contents: vec![
///                 Content::Text(TextContent::new("Hello, world!"))
///             ],
///         }
///     ],
///     meta: ResponseMetadata::default(),
/// };
/// ```
///
/// # JSON Representation
///
/// ```json
/// {
///   "contents": [
///     {
///       "type": "text",
///       "content": "Hello, world!",
///       "mimeType": "text/plain",
///       "uri": "file:///document.txt"
///     }
///   ]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesReadResponse {
    /// The contents of the requested resource, which may include different representations
    /// or chunks of the same resource
    pub contents: Vec<ResourceContent>,

    /// Additional metadata associated with the response
    #[serde(flatten)]
    pub meta: ResponseMetadata,
}

/// Request to subscribe to updates for a specific resource.
///
/// This request corresponds to the `resources/subscribe` method in the MCP specification.
/// It establishes a subscription to receive notifications when the resource is updated.
///
/// # Fields
///
/// * `uri` - The unique identifier (URI) of the resource to subscribe to
///
/// # Example
///
/// ```rust
/// use mocopr_core::types::ResourcesSubscribeRequest;
/// use url::Url;
///
/// let request = ResourcesSubscribeRequest {
///     uri: Url::parse("file:///document.txt").unwrap(),
/// };
/// ```
///
/// # JSON Representation
///
/// ```json
/// {
///   "uri": "file:///document.txt"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesSubscribeRequest {
    /// The unique identifier (URI) of the resource to subscribe to
    pub uri: Url,
}

/// Response returned by the `resources/subscribe` method confirming the subscription.
///
/// This response confirms that the subscription was established successfully.
/// Subsequent updates to the subscribed resource will be sent as notifications.
///
/// # Fields
///
/// * `meta` - Additional metadata associated with the response
///
/// # Example
///
/// ```rust
/// use mocopr_core::types::{ResourcesSubscribeResponse, ResponseMetadata};
///
/// let response = ResourcesSubscribeResponse {
///     meta: ResponseMetadata::default(),
/// };
/// ```
///
/// # JSON Representation
///
/// ```json
/// {}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesSubscribeResponse {
    /// Additional metadata associated with the response
    #[serde(flatten)]
    pub meta: ResponseMetadata,
}

/// Request to unsubscribe from updates for a specific resource.
///
/// This request corresponds to the `resources/unsubscribe` method in the MCP specification.
/// It cancels a subscription that was previously established for the resource.
///
/// # Fields
///
/// * `uri` - The unique identifier (URI) of the resource to unsubscribe from
///
/// # Example
///
/// ```rust
/// use mocopr_core::types::ResourcesUnsubscribeRequest;
/// use url::Url;
///
/// let request = ResourcesUnsubscribeRequest {
///     uri: Url::parse("file:///document.txt").unwrap(),
/// };
/// ```
///
/// # JSON Representation
///
/// ```json
/// {
///   "uri": "file:///document.txt"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesUnsubscribeRequest {
    /// The unique identifier (URI) of the resource to unsubscribe from
    pub uri: Url,
}

/// Response returned by the `resources/unsubscribe` method confirming the cancellation of a subscription.
///
/// This response confirms that the subscription was canceled successfully.
/// No further notifications will be sent for updates to the unsubscribed resource.
///
/// # Fields
///
/// * `meta` - Additional metadata associated with the response
///
/// # Example
///
/// ```rust
/// use mocopr_core::types::{ResourcesUnsubscribeResponse, ResponseMetadata};
///
/// let response = ResourcesUnsubscribeResponse {
///     meta: ResponseMetadata::default(),
/// };
/// ```
///
/// # JSON Representation
///
/// ```json
/// {}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesUnsubscribeResponse {
    /// Additional metadata associated with the response
    #[serde(flatten)]
    pub meta: ResponseMetadata,
}

/// Notification sent when the available resource list has changed.
///
/// This notification corresponds to the `resources/listChanged` notification in the MCP specification.
/// It informs clients that the list of available resources has been modified (resources added, removed,
/// or updated) and that clients should refresh their resource listings.
///
/// This notification contains no additional fields beyond what's required for the MCP notification format.
///
/// # Example
///
/// ```rust
/// use mocopr_core::types::ResourcesListChangedNotification;
///
/// let notification = ResourcesListChangedNotification {};
/// ```
///
/// # JSON Representation
///
/// ```json
/// {}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesListChangedNotification {
    // No additional fields required
}

/// Notification sent when a specific resource has been updated.
///
/// This notification corresponds to the `resources/updated` notification in the MCP specification.
/// It informs clients that have subscribed to a specific resource that the content of that
/// resource has changed. Clients can then retrieve the updated content using a `resources/read` request.
///
/// # Fields
///
/// * `uri` - The unique identifier (URI) of the resource that was updated
///
/// # Example
///
/// ```rust
/// use mocopr_core::types::ResourcesUpdatedNotification;
/// use url::Url;
///
/// let notification = ResourcesUpdatedNotification {
///     uri: Url::parse("file:///document.txt").unwrap(),
/// };
/// ```
///
/// # JSON Representation
///
/// ```json
/// {
///   "uri": "file:///document.txt"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesUpdatedNotification {
    /// The unique identifier (URI) of the resource that was updated
    pub uri: Url,
}

impl Resource {
    /// Create a new resource with validation.
    ///
    /// This method validates the URI scheme and other security aspects
    /// before creating a resource.
    ///
    /// # Security
    ///
    /// This method validates that:
    /// - The URI scheme is allowed
    /// - The resource name contains only safe characters
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI identifying the resource
    /// * `name` - The human-readable name for the resource
    /// * `allowed_schemes` - List of URI schemes that are allowed (e.g., `["file", "http", "https"]`)
    ///
    /// # Returns
    ///
    /// A `Result` containing either the new `Resource` or an error if validation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The URI scheme is not in the allowed schemes list
    /// - The resource name contains invalid characters
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::types::Resource;
    /// use url::Url;
    ///
    /// let uri = Url::parse("file:///path/to/resource.txt").unwrap();
    /// let resource = Resource::new_validated(uri, "My Resource", &["file"]).unwrap();
    ///
    /// // This would fail validation:
    /// // let uri = Url::parse("http://example.com/resource").unwrap();
    /// // let result = Resource::new_validated(uri, "My Resource", &["file"]);
    /// // assert!(result.is_err());
    /// ```
    pub fn new_validated(
        uri: Url,
        name: impl Into<String>,
        allowed_schemes: &[&str],
    ) -> crate::Result<Self> {
        let name_str: String = name.into();

        // Validate URI scheme
        crate::utils::Utils::validate_uri_scheme(&uri, allowed_schemes)?;

        // Validate resource name
        crate::utils::Utils::validate_safe_string(&name_str)?;

        Ok(Self {
            uri,
            name: name_str,
            description: None,
            mime_type: None,
            annotations: None,
        })
    }

    /// Creates a new resource with the given URI and name.
    ///
    /// This is a convenience method that creates a resource without validation.
    /// For security-sensitive applications, consider using [`new_validated`](#method.new_validated) instead.
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI identifying the resource
    /// * `name` - The human-readable name for the resource
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::types::Resource;
    /// use url::Url;
    ///
    /// let uri = Url::parse("file:///path/to/resource.txt").unwrap();
    /// let resource = Resource::new(uri, "My Resource");
    ///
    /// assert_eq!(resource.name, "My Resource");
    /// assert!(resource.description.is_none());
    /// ```
    pub fn new(uri: Url, name: impl Into<String>) -> Self {
        Self {
            uri,
            name: name.into(),
            description: None,
            mime_type: None,
            annotations: None,
        }
    }

    /// Sets the description for this resource.
    ///
    /// This method follows the builder pattern and returns `self` for method chaining.
    ///
    /// # Arguments
    ///
    /// * `description` - A human-readable description of the resource's purpose or contents
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::types::Resource;
    /// use url::Url;
    ///
    /// let uri = Url::parse("file:///path/to/resource.txt").unwrap();
    /// let resource = Resource::new(uri, "My Resource")
    ///     .with_description("A text file containing important data");
    ///
    /// assert_eq!(resource.description.unwrap(), "A text file containing important data");
    /// ```
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the MIME type for this resource.
    ///
    /// This method follows the builder pattern and returns `self` for method chaining.
    ///
    /// # Arguments
    ///
    /// * `mime_type` - The MIME type of the resource (e.g., "text/plain", "application/json")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::types::Resource;
    /// use url::Url;
    ///
    /// let uri = Url::parse("file:///path/to/resource.json").unwrap();
    /// let resource = Resource::new(uri, "Config File")
    ///     .with_mime_type("application/json");
    ///
    /// assert_eq!(resource.mime_type.unwrap(), "application/json");
    /// ```
    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Sets custom annotations for this resource.
    ///
    /// Annotations provide additional metadata about the resource that may be useful
    /// for specific applications or use cases.
    ///
    /// This method follows the builder pattern and returns `self` for method chaining.
    ///
    /// # Arguments
    ///
    /// * `annotations` - A JSON value containing custom metadata
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::types::Resource;
    /// use url::Url;
    /// use serde_json::json;
    ///
    /// let uri = Url::parse("file:///path/to/data.csv").unwrap();
    /// let resource = Resource::new(uri, "Data File")
    ///     .with_annotations(json!({
    ///         "rows": 1000,
    ///         "columns": 5,
    ///         "format": "CSV",
    ///         "has_header": true
    ///     }));
    ///
    /// // Access annotations
    /// if let Some(annotations) = &resource.annotations {
    ///     if let Some(rows) = annotations.get("rows") {
    ///         assert_eq!(rows.as_u64().unwrap(), 1000);
    ///     }
    /// }
    /// ```
    pub fn with_annotations(mut self, annotations: serde_json::Value) -> Self {
        // Simply set the annotations directly
        self.annotations = Some(annotations);
        self
    }

    /// Validate this resource's security properties.
    ///
    /// Checks that the URI scheme is allowed and all text fields are safe.
    ///
    /// # Arguments
    ///
    /// * `allowed_schemes` - List of allowed URI schemes (e.g., &["file", "http", "https"])
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::types::Resource;
    /// use url::Url;
    ///
    /// let resource = Resource::new(
    ///     Url::parse("file:///safe/path.txt").unwrap(),
    ///     "safe_file.txt"
    /// );
    /// resource.validate_security(&["file", "http", "https"]).unwrap();
    /// ```
    pub fn validate_security(&self, allowed_schemes: &[&str]) -> crate::Result<()> {
        // Validate URI scheme
        crate::utils::Utils::validate_uri_scheme(&self.uri, allowed_schemes)?;

        // Validate resource name
        crate::utils::Utils::validate_safe_string(&self.name)?;

        // Validate description if present
        if let Some(ref desc) = self.description {
            crate::utils::Utils::validate_safe_string(desc)?;
        }

        // Validate MIME type if present
        if let Some(ref mime) = self.mime_type {
            crate::utils::Utils::validate_safe_string(mime)?;
        }

        Ok(())
    }
}

impl ResourceContent {
    /// Creates a new resource content instance with the given URI and contents.
    ///
    /// This method initializes a `ResourceContent` with a URI and content data,
    /// without setting a MIME type. You can use the builder pattern with
    /// `with_mime_type` to add a MIME type.
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI identifying the source of the content
    /// * `contents` - A vector of content pieces (text, binary, etc.)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::types::{ResourceContent, Content};
    /// use url::Url;
    ///
    /// let uri = Url::parse("file:///example.txt").unwrap();
    /// let content = ResourceContent::new(
    ///     uri,
    ///     vec![Content::from("Hello, world!")]
    /// );
    /// ```
    pub fn new(uri: Url, contents: Vec<Content>) -> Self {
        Self {
            uri,
            mime_type: None,
            contents,
        }
    }

    /// Sets the MIME type for this resource content.
    ///
    /// This method follows the builder pattern and returns `self` for method chaining.
    ///
    /// # Arguments
    ///
    /// * `mime_type` - The MIME type of the content (e.g., "text/plain", "application/json")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::types::{ResourceContent, Content};
    /// use url::Url;
    ///
    /// let uri = Url::parse("file:///example.json").unwrap();
    /// let content = ResourceContent::new(
    ///     uri,
    ///     vec![Content::from(r#"{"key": "value"}"#)]
    /// ).with_mime_type("application/json");
    ///
    /// assert_eq!(content.mime_type, Some("application/json".to_string()));
    /// ```
    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }
}

impl ResourcesListRequest {
    /// Creates a new request to list available resources.
    ///
    /// This method initializes a `ResourcesListRequest` with default pagination settings
    /// (no cursor, using server default limits).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::types::ResourcesListRequest;
    ///
    /// let request = ResourcesListRequest::new();
    /// assert!(request.pagination.cursor.is_none());
    /// ```
    pub fn new() -> Self {
        Self {
            pagination: PaginationParams { cursor: None },
        }
    }

    /// Sets a pagination cursor for this request.
    ///
    /// This method follows the builder pattern and returns `self` for method chaining.
    /// The cursor is typically obtained from a previous response's `next_cursor` field
    /// and is used to paginate through a large list of resources.
    ///
    /// # Arguments
    ///
    /// * `cursor` - The pagination cursor to use for the request
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mocopr_core::types::ResourcesListRequest;
    ///
    /// let request = ResourcesListRequest::new()
    ///     .with_cursor("next-page-token");
    ///
    /// assert_eq!(request.pagination.cursor, Some("next-page-token".to_string()));
    /// ```
    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.pagination.cursor = Some(cursor.into());
        self
    }
}

/// Default implementation that creates a new request with no pagination cursor.
///
/// This is equivalent to calling `ResourcesListRequest::new()`.
///
/// # Examples
///
/// ```rust
/// use mocopr_core::types::ResourcesListRequest;
///
/// let request = ResourcesListRequest::default();
/// assert!(request.pagination.cursor.is_none());
/// ```
impl Default for ResourcesListRequest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_creation() {
        let resource = Resource {
            uri: url::Url::parse("file:///test.txt").unwrap(),
            name: "Test Resource".to_string(),
            description: Some("A test resource".to_string()),
            mime_type: Some("text/plain".to_string()),
            annotations: None,
        };

        assert_eq!(resource.name, "Test Resource");
        assert_eq!(resource.description, Some("A test resource".to_string()));
    }

    #[test]
    fn test_resource_serialization() {
        let resource = Resource {
            uri: url::Url::parse("https://example.com/test").unwrap(),
            name: "API Resource".to_string(),
            description: None,
            mime_type: Some("application/json".to_string()),
            annotations: None,
        };

        let serialized = serde_json::to_string(&resource).unwrap();
        let deserialized: Resource = serde_json::from_str(&serialized).unwrap();

        assert_eq!(resource.name, deserialized.name);
        assert_eq!(resource.mime_type, deserialized.mime_type);
    }

    #[test]
    fn test_resources_list_request() {
        let request = ResourcesListRequest::new();
        assert!(request.pagination.cursor.is_none());

        let request_with_cursor = request.with_cursor("test_cursor");
        assert_eq!(
            request_with_cursor.pagination.cursor,
            Some("test_cursor".to_string())
        );
    }

    #[test]
    fn test_read_resource_request() {
        let request = ResourcesReadRequest {
            uri: url::Url::parse("file:///important.txt").unwrap(),
        };

        assert_eq!(request.uri.as_str(), "file:///important.txt");
    }
}
