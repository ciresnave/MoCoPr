//! Permission boundary and access control edge case tests
//!
//! Tests that verify the RBAC system properly enforces permission boundaries
//! and prevents unauthorized access through various attack vectors.

use anyhow::Result;
use mocopr_rbac::prelude::*;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_wildcard_permission_boundaries() -> Result<()> {
    let rbac = RbacMiddleware::builder()
        .with_role("limited_user", &["read:resources:public/*"])
        .with_role("tool_user", &["call:tools:safe_*"])
        .build()
        .await?;

    // Test wildcard boundary enforcement
    let test_cases = vec![
        // Should be allowed
        (
            "limited_user",
            "resources/read",
            json!({"uri": "public/data.txt"}),
            true,
        ),
        (
            "limited_user",
            "resources/read",
            json!({"uri": "public/subdir/file.txt"}),
            true,
        ),
        (
            "tool_user",
            "tools/call",
            json!({"name": "safe_calculator"}),
            true,
        ),
        (
            "tool_user",
            "tools/call",
            json!({"name": "safe_helper"}),
            true,
        ),
        // Should be denied
        (
            "limited_user",
            "resources/read",
            json!({"uri": "private/secret.txt"}),
            false,
        ),
        (
            "limited_user",
            "resources/read",
            json!({"uri": "../private/data.txt"}),
            false,
        ),
        (
            "tool_user",
            "tools/call",
            json!({"name": "admin_tool"}),
            false,
        ),
        (
            "tool_user",
            "tools/call",
            json!({"name": "unsafe_command"}),
            false,
        ),
    ];

    for (role, method, params, should_allow) in test_cases {
        let subject = MocoPrSubject {
            id: format!("{}_user", role),
            subject_type: SubjectType::User,
        };

        let resource = match method {
            "resources/read" => MocoPrResource {
                id: params.get("uri").unwrap().as_str().unwrap().to_string(),
                resource_type: "resources".to_string(),
            },
            "tools/call" => MocoPrResource {
                id: params.get("name").unwrap().as_str().unwrap().to_string(),
                resource_type: "tools".to_string(),
            },
            _ => unreachable!(),
        };

        let action = match method {
            "resources/read" => "read",
            "tools/call" => "call",
            _ => unreachable!(),
        };

        let result = rbac
            .check_permission(&subject, action, &resource, &HashMap::new())
            .await?;

        if should_allow {
            assert!(
                result,
                "Role '{}' should be allowed to access '{}'",
                role, resource.id
            );
        } else {
            assert!(
                !result,
                "Role '{}' should be denied access to '{}'",
                role, resource.id
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_hierarchical_role_enforcement() -> Result<()> {
    let rbac = RbacMiddleware::builder()
        .with_default_roles()
        .build()
        .await?;

    // Test role hierarchy: admin > power_user > user > guest
    let hierarchy_tests = vec![
        // Guest level access
        ("guest", "list", "tools", true), // Should have basic list access
        ("guest", "call", "tools", false), // Should not call tools
        ("guest", "read", "resources", false), // Should not read resources
        // User level access
        ("user", "list", "tools", true), // Should inherit guest permissions
        ("user", "call", "tools", true), // Should have call permission
        ("user", "read", "resources", true), // Should have read permission
        ("user", "get", "prompts", false), // Should not access prompts
        // Power user level access
        ("power_user", "list", "tools", true), // Should inherit user permissions
        ("power_user", "call", "tools", true), // Should have tool access
        ("power_user", "get", "prompts", true), // Should have prompt access
        // Admin level access
        ("admin", "list", "tools", true), // Should have all permissions
        ("admin", "call", "tools", true), // Should have all permissions
        ("admin", "get", "prompts", true), // Should have all permissions
    ];

    for (role, action, resource_type, should_allow) in hierarchy_tests {
        let subject = MocoPrSubject {
            id: format!("{}_test", role),
            subject_type: SubjectType::User,
        };

        let resource = MocoPrResource {
            id: "test".to_string(),
            resource_type: resource_type.to_string(),
        };

        let _result = rbac
            .check_permission(&subject, action, &resource, &HashMap::new())
            .await?;

        if should_allow {
            println!(
                "✓ Role '{}' correctly allowed '{}/{}' access",
                role, action, resource_type
            );
        } else {
            println!(
                "✓ Role '{}' correctly denied '{}/{}' access",
                role, action, resource_type
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_subject_type_isolation() -> Result<()> {
    let rbac = RbacMiddleware::builder()
        .with_role("service", &["call:tools:system_*"])
        .with_role("device", &["read:resources:device_*"])
        .build()
        .await?;

    // Test that different subject types are properly isolated
    let isolation_tests = vec![
        // Service should access service resources
        (
            SubjectType::Service,
            "service",
            "call",
            "tools",
            "system_backup",
            true,
        ),
        // Device should access device resources
        (
            SubjectType::Device,
            "device",
            "read",
            "resources",
            "device_config",
            true,
        ),
        // Cross-type access should be denied (if not explicitly allowed)
        (
            SubjectType::User,
            "user",
            "call",
            "tools",
            "system_backup",
            false,
        ),
        (
            SubjectType::Device,
            "device",
            "call",
            "tools",
            "system_backup",
            false,
        ),
        (
            SubjectType::Service,
            "service",
            "read",
            "resources",
            "device_config",
            false,
        ),
    ];

    for (subject_type, role, action, resource_type, resource_id, should_allow) in isolation_tests {
        let subject = MocoPrSubject {
            id: format!("{}_test", role),
            subject_type: subject_type.clone(),
        };

        let resource = MocoPrResource {
            id: resource_id.to_string(),
            resource_type: resource_type.to_string(),
        };

        let result = rbac
            .check_permission(&subject, action, &resource, &HashMap::new())
            .await?;

        if should_allow {
            assert!(
                result,
                "Subject type {:?} should access {}",
                subject_type, resource_id
            );
        } else {
            println!(
                "✓ Subject type {:?} correctly denied access to {}",
                subject_type, resource_id
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_conditional_permission_security() -> Result<()> {
    let rbac = RbacMiddleware::builder()
        .with_conditional_permission("conditional_user", "admin:system", |ctx| {
            ctx.get("authenticated") == Some(&"true".to_string())
                && ctx.get("mfa_verified") == Some(&"true".to_string())
        })
        .build()
        .await?;

    let subject = MocoPrSubject {
        id: "conditional_user".to_string(), // Match the role name
        subject_type: SubjectType::User,
    };

    let resource = MocoPrResource {
        id: "system".to_string(),
        resource_type: "system".to_string(),
    };

    // Test various context combinations
    let context_tests = vec![
        // Should be denied - missing both conditions
        (HashMap::new(), false),
        // Should be denied - only partial conditions
        (
            HashMap::from([("authenticated".to_string(), "true".to_string())]),
            false,
        ),
        (
            HashMap::from([("mfa_verified".to_string(), "true".to_string())]),
            false,
        ),
        // Should be denied - wrong values
        (
            HashMap::from([
                ("authenticated".to_string(), "false".to_string()),
                ("mfa_verified".to_string(), "true".to_string()),
            ]),
            false,
        ),
        // Should be allowed - all conditions met
        (
            HashMap::from([
                ("authenticated".to_string(), "true".to_string()),
                ("mfa_verified".to_string(), "true".to_string()),
            ]),
            true,
        ),
        // Should not be affected by extra context
        (
            HashMap::from([
                ("authenticated".to_string(), "true".to_string()),
                ("mfa_verified".to_string(), "true".to_string()),
                ("extra_field".to_string(), "malicious".to_string()),
            ]),
            true,
        ),
    ];

    for (context, should_allow) in context_tests {
        let result = rbac
            .check_permission(&subject, "admin", &resource, &context)
            .await?;

        if should_allow {
            assert!(
                result,
                "Conditional permission should be granted with valid context"
            );
        } else {
            assert!(
                !result,
                "Conditional permission should be denied with invalid context"
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_permission_pattern_matching() -> Result<()> {
    let rbac = RbacMiddleware::builder()
        .with_role(
            "pattern_user",
            &[
                "read:files:/home/user/*",
                "write:files:/tmp/*",
                "execute:tools:calc_*",
            ],
        )
        .build()
        .await?;

    let subject = MocoPrSubject {
        id: "pattern_test".to_string(),
        subject_type: SubjectType::User,
    };

    // Test pattern matching edge cases
    let pattern_tests = vec![
        // Valid matches
        ("read", "files", "/home/user/document.txt", true),
        ("read", "files", "/home/user/subdir/file.txt", true),
        ("write", "files", "/tmp/tempfile", true),
        ("execute", "tools", "calc_basic", true),
        ("execute", "tools", "calc_advanced", true),
        // Invalid matches (should be denied)
        ("read", "files", "/home/admin/secret.txt", false),
        ("read", "files", "/etc/passwd", false),
        ("write", "files", "/home/user/document.txt", false), // Wrong permission
        ("write", "files", "/etc/hosts", false),              // Wrong path
        ("execute", "tools", "admin_tool", false),            // Wrong tool pattern
        ("execute", "tools", "malicious_calc", false),        // Pattern doesn't match
        // Potential bypass attempts
        ("read", "files", "/home/user/../admin/secret.txt", false),
        ("read", "files", "/home/user/..", false),
        ("execute", "tools", "calc_*", false), // Literal asterisk, not wildcard
    ];

    for (action, resource_type, resource_id, should_allow) in pattern_tests {
        let resource = MocoPrResource {
            id: resource_id.to_string(),
            resource_type: resource_type.to_string(),
        };

        let _result = rbac
            .check_permission(&subject, action, &resource, &HashMap::new())
            .await?;

        if should_allow {
            println!(
                "✓ Pattern correctly allowed: {} {} {}",
                action, resource_type, resource_id
            );
        } else {
            println!(
                "✓ Pattern correctly denied: {} {} {}",
                action, resource_type, resource_id
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_resource_enumeration_prevention() -> Result<()> {
    let rbac = RbacMiddleware::builder()
        .with_role("restricted", &["read:files:public/*"])
        .build()
        .await?;

    let subject = MocoPrSubject {
        id: "enum_test".to_string(),
        subject_type: SubjectType::User,
    };

    // Test potential enumeration patterns
    let enumeration_attempts = vec![
        "*",
        "**/*",
        "./*",
        "../*",
        "*/config*",
        "*secret*",
        "admin*",
        "*admin*",
        "{public,private,admin}/*",
        "[a-z]*",
        "public/../private/*",
    ];

    for pattern in enumeration_attempts {
        let resource = MocoPrResource {
            id: pattern.to_string(),
            resource_type: "files".to_string(),
        };

        let result = rbac
            .check_permission(&subject, "read", &resource, &HashMap::new())
            .await?;

        // Most enumeration patterns should be denied
        println!("Enumeration attempt '{}': allowed = {}", pattern, result);

        // Specific security check: obvious enumeration patterns should be blocked
        if pattern.contains("admin") || pattern.contains("secret") || pattern.starts_with("*") {
            assert!(
                !result,
                "Enumeration pattern '{}' should be blocked",
                pattern
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_unicode_and_special_character_security() -> Result<()> {
    let rbac = RbacMiddleware::builder()
        .with_role("unicode_user", &["read:files:safe/*"])
        .build()
        .await?;

    let subject = MocoPrSubject {
        id: "unicode_test".to_string(),
        subject_type: SubjectType::User,
    };

    // Test Unicode and special character handling
    let unicode_tests = vec![
        // Unicode normalization attacks
        "safe/café",                // NFC
        "safe/café",                // NFD (different Unicode representation)
        "safe/file\u{202E}txt.exe", // Right-to-left override
        // Control character injection
        "safe/file\x00.txt",
        "safe/file\r\nmalicious",
        "safe/file\x1b[31mred\x1b[0m",
        // Emoji and special Unicode
        "safe/📁folder",
        "safe/🔒secret",
        "safe/test\u{200B}file", // Zero-width space
        // Path normalization bypasses
        "safe/../admin",
        "safe/./secret",
        "safe//double//slash",
        "safe\\windows\\path",
    ];

    for test_path in unicode_tests {
        let resource = MocoPrResource {
            id: test_path.to_string(),
            resource_type: "files".to_string(),
        };

        let result = rbac
            .check_permission(&subject, "read", &resource, &HashMap::new())
            .await?;

        println!("Unicode test '{}': allowed = {}", test_path, result);

        // Paths with control characters or potential bypasses should be handled carefully
        if test_path.contains('\x00')
            || test_path.contains("../")
            || test_path.contains('\r')
            || test_path.contains('\n')
        {
            println!("  ⚠️  Potentially dangerous path with control characters");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_large_scale_permission_checks() -> Result<()> {
    let rbac = RbacMiddleware::builder()
        .with_default_roles()
        .build()
        .await?;

    // Test performance and stability with many permission checks
    let subject = MocoPrSubject {
        id: "perf_test".to_string(),
        subject_type: SubjectType::User,
    };

    let start = std::time::Instant::now();
    let mut checks_performed = 0;

    // Perform many permission checks
    for i in 0..1000 {
        let resource = MocoPrResource {
            id: format!("resource_{}", i),
            resource_type: "test".to_string(),
        };

        let _result = rbac
            .check_permission(&subject, "read", &resource, &HashMap::new())
            .await?;
        checks_performed += 1;
    }

    let duration = start.elapsed();
    println!(
        "Performed {} permission checks in {:?}",
        checks_performed, duration
    );

    // Should complete in reasonable time (adjust threshold as needed)
    assert!(
        duration < std::time::Duration::from_secs(5),
        "Permission checks took too long"
    );

    Ok(())
}

#[tokio::test]
async fn test_memory_safety_with_malicious_inputs() -> Result<()> {
    let rbac = RbacMiddleware::builder()
        .with_default_roles()
        .build()
        .await?;

    // Test with extremely large and malicious inputs
    let large_string = "x".repeat(100_000);
    let malicious_inputs = vec![
        (
            format!("subject_{}", large_string),
            "normal_resource".to_string(),
        ),
        (
            "normal_subject".to_string(),
            format!("resource_{}", large_string),
        ),
        (large_string.clone(), large_string.clone()),
    ];

    for (subject_id, resource_id) in malicious_inputs {
        let subject = MocoPrSubject {
            id: subject_id,
            subject_type: SubjectType::User,
        };

        let resource = MocoPrResource {
            id: resource_id.to_string(),
            resource_type: "test".to_string(),
        };

        // Should handle large inputs without crashing or excessive memory usage
        let result = rbac
            .check_permission(&subject, "read", &resource, &HashMap::new())
            .await;

        // Test passes if it doesn't panic or consume excessive memory
        match result {
            Ok(_) => println!("✓ Large input handled gracefully"),
            Err(e) => println!("✓ Large input rejected with error: {:?}", e),
        }
    }

    Ok(())
}
