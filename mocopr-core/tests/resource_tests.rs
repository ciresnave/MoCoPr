//! Comprehensive tests for the Resource type
//!
//! These tests focus on ensuring the Resource type behaves correctly
//! under all circumstances, including edge cases and error conditions.

use mocopr_core::types::Resource;
use serde_json::json;
use url::Url;

#[test]
fn test_resource_creation_basic() {
    let uri = Url::parse("file:///path/to/resource.txt").unwrap();
    let resource = Resource::new(uri.clone(), "Test Resource");

    assert_eq!(resource.uri, uri);
    assert_eq!(resource.name, "Test Resource");
    assert!(resource.description.is_none());
    assert!(resource.mime_type.is_none());
    assert!(resource.annotations.is_none());
}

#[test]
fn test_resource_builder_pattern() {
    let uri = Url::parse("file:///path/to/resource.txt").unwrap();
    let resource = Resource::new(uri.clone(), "Test Resource")
        .with_description("A test resource")
        .with_mime_type("text/plain")
        .with_annotations(json!({"creator": "test", "version": 1}));

    assert_eq!(resource.uri, uri);
    assert_eq!(resource.name, "Test Resource");
    assert_eq!(resource.description, Some("A test resource".to_string()));
    assert_eq!(resource.mime_type, Some("text/plain".to_string()));

    let annotations = resource.annotations.unwrap();
    assert_eq!(annotations["creator"], "test");
    assert_eq!(annotations["version"], 1);
}

#[test]
fn test_resource_serialization_roundtrip() {
    let uri = Url::parse("file:///path/to/resource.txt").unwrap();
    let original = Resource::new(uri, "Test Resource")
        .with_description("A test resource")
        .with_mime_type("text/plain");

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: Resource = serde_json::from_str(&serialized).unwrap();

    assert_eq!(original.uri, deserialized.uri);
    assert_eq!(original.name, deserialized.name);
    assert_eq!(original.description, deserialized.description);
    assert_eq!(original.mime_type, deserialized.mime_type);
}

#[test]
fn test_resource_validation_allowed_scheme() {
    let uri = Url::parse("file:///path/to/resource.txt").unwrap();
    let result = Resource::new_validated(uri, "Valid Resource", &["file"]);
    assert!(result.is_ok());
}

#[test]
fn test_resource_validation_disallowed_scheme() {
    let uri = Url::parse("http://example.com/resource").unwrap();
    let result = Resource::new_validated(uri, "Invalid Resource", &["file"]);
    assert!(result.is_err());

    // Check error message contains useful info
    let err = result.unwrap_err().to_string();
    assert!(err.contains("scheme"));
    assert!(err.contains("http"));
}

#[test]
fn test_resource_validation_unsafe_name() {
    let uri = Url::parse("file:///path/to/resource.txt").unwrap();
    // Create a name with a control character
    let name = format!("Bad{}Name", char::from(1)); // SOH control character

    let result = Resource::new_validated(uri, name, &["file"]);
    assert!(result.is_err());

    // Check error message contains useful info
    let err = result.unwrap_err().to_string();
    assert!(err.contains("unsafe"));
}

#[test]
fn test_resource_security_validation() {
    let uri = Url::parse("file:///path/to/resource.txt").unwrap();
    let resource = Resource::new(uri, "Test Resource")
        .with_description("A test resource")
        .with_mime_type("text/plain");

    // Should pass with allowed scheme
    assert!(resource.validate_security(&["file"]).is_ok());

    // Should fail with disallowed scheme
    assert!(resource.validate_security(&["http", "https"]).is_err());
}

#[test]
fn test_resource_security_validation_with_unsafe_fields() {
    // Test with unsafe description
    let uri = Url::parse("file:///path/to/resource.txt").unwrap();
    let description_with_control = format!("Bad{}Description", char::from(0)); // NULL control character
    let resource =
        Resource::new(uri.clone(), "Test Resource").with_description(description_with_control);

    assert!(resource.validate_security(&["file"]).is_err());

    // Test with unsafe mime type
    let mime_with_control = format!("text/{}plain", char::from(31)); // US control character
    let resource = Resource::new(uri, "Test Resource").with_mime_type(mime_with_control);

    assert!(resource.validate_security(&["file"]).is_err());
}

#[test]
fn test_resource_with_empty_values() {
    let uri = Url::parse("file:///path/to/resource.txt").unwrap();

    // Empty name
    let resource = Resource::new(uri.clone(), "");
    assert_eq!(resource.name, "");

    // Empty description
    let resource = Resource::new(uri.clone(), "Test Resource").with_description("");
    assert_eq!(resource.description, Some("".to_string()));

    // Empty MIME type
    let resource = Resource::new(uri, "Test Resource").with_mime_type("");
    assert_eq!(resource.mime_type, Some("".to_string()));
}

#[test]
fn test_resource_with_complex_annotations() {
    let uri = Url::parse("file:///path/to/resource.txt").unwrap();

    let complex_annotations = json!({
        "metadata": {
            "created": "2025-06-18T12:30:00Z",
            "size": 1024,
            "tags": ["important", "documentation"],
            "permissions": {
                "read": true,
                "write": false
            }
        },
        "versions": [
            {"id": 1, "timestamp": "2025-06-17T10:00:00Z"},
            {"id": 2, "timestamp": "2025-06-18T12:30:00Z"}
        ]
    });

    let resource =
        Resource::new(uri, "Complex Resource").with_annotations(complex_annotations.clone());

    let annotations = resource.annotations.unwrap();
    assert_eq!(annotations["metadata"]["created"], "2025-06-18T12:30:00Z");
    assert_eq!(annotations["metadata"]["size"], 1024);
    assert_eq!(annotations["versions"][1]["id"], 2);
}

#[test]
fn test_resource_uri_edge_cases() {
    // URI with query parameters
    let uri = Url::parse("file:///path/to/resource.txt?param=value").unwrap();
    let resource = Resource::new(uri.clone(), "Resource with Query");
    assert_eq!(resource.uri, uri);

    // URI with fragment
    let uri = Url::parse("file:///path/to/resource.txt#section1").unwrap();
    let resource = Resource::new(uri.clone(), "Resource with Fragment");
    assert_eq!(resource.uri, uri);

    // URI with special characters
    let uri = Url::parse("file:///path/to/resource%20with%20spaces.txt").unwrap();
    let resource = Resource::new(uri.clone(), "Resource with Spaces");
    assert_eq!(resource.uri, uri);
}
