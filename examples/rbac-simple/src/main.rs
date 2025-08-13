//! Simple RBAC Concepts Example
//!
//! This example demonstrates the conceptual integration of RBAC with MoCoPr
//! without relying on complex role-system APIs that might not be stable.

#![allow(dead_code)]

use std::collections::HashMap;

#[derive(Debug, Clone)]
struct SimpleRole {
    name: String,
    permissions: Vec<String>,
}

#[derive(Debug, Clone)]
struct SimpleSubject {
    id: String,
    roles: Vec<String>,
}

#[derive(Debug)]
struct SimpleRBAC {
    roles: HashMap<String, SimpleRole>,
    subjects: HashMap<String, SimpleSubject>,
}

impl SimpleRBAC {
    fn new() -> Self {
        Self {
            roles: HashMap::new(),
            subjects: HashMap::new(),
        }
    }

    fn add_role(&mut self, name: &str, permissions: Vec<&str>) {
        let role = SimpleRole {
            name: name.to_string(),
            permissions: permissions.iter().map(|p| p.to_string()).collect(),
        };
        self.roles.insert(name.to_string(), role);
    }

    fn add_subject(&mut self, id: &str, roles: Vec<&str>) {
        let subject = SimpleSubject {
            id: id.to_string(),
            roles: roles.iter().map(|r| r.to_string()).collect(),
        };
        self.subjects.insert(id.to_string(), subject);
    }

    fn check_permission(&self, subject_id: &str, permission: &str) -> bool {
        if let Some(subject) = self.subjects.get(subject_id) {
            for role_name in &subject.roles {
                if let Some(role) = self.roles.get(role_name) {
                    if role.permissions.contains(&permission.to_string()) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 MoCoPr RBAC Integration Concepts");
    println!("===================================");

    // Create a simple RBAC system for demonstration
    let mut rbac = SimpleRBAC::new();

    // Define roles with MCP-specific permissions
    rbac.add_role("guest", vec!["list:tools", "read:resources:public"]);

    rbac.add_role(
        "user",
        vec![
            "list:tools",
            "call:tools:calculator",
            "read:resources:public",
            "read:resources:user",
            "list:prompts",
        ],
    );

    rbac.add_role(
        "admin",
        vec![
            "list:tools",
            "call:tools:*",
            "read:resources:*",
            "write:resources:*",
            "list:prompts",
            "get:prompts:*",
            "admin:system",
        ],
    );

    // Add subjects with different role assignments
    rbac.add_subject("guest_001", vec!["guest"]);
    rbac.add_subject("user_001", vec!["user"]);
    rbac.add_subject("admin_001", vec!["admin"]);
    rbac.add_subject("power_user_001", vec!["user", "guest"]); // Multiple roles

    println!("\n📋 Permission Check Results:");
    println!("----------------------------");

    // Test different permissions for different subjects
    let test_cases = [
        ("guest_001", "list:tools", "Guest listing tools"),
        (
            "guest_001",
            "call:tools:calculator",
            "Guest calling calculator",
        ),
        (
            "user_001",
            "call:tools:calculator",
            "User calling calculator",
        ),
        ("user_001", "admin:system", "User doing admin tasks"),
        ("admin_001", "admin:system", "Admin doing admin tasks"),
        ("power_user_001", "list:tools", "Power user listing tools"),
    ];

    for (subject_id, permission, description) in test_cases {
        let result = rbac.check_permission(subject_id, permission);
        let icon = if result { "✅" } else { "❌" };
        println!("  {icon} {description}: {result}");
    }

    println!("\n🛡️ MoCoPr Integration Architecture:");
    println!("-----------------------------------");
    println!("1. RbacMiddleware intercepts MCP requests");
    println!("2. Extracts subject from context (API key, JWT, session)");
    println!("3. Maps MCP operation to permission:");
    println!("   • tools/list     → list:tools");
    println!("   • tools/call     → call:tools:{{tool_name}}");
    println!("   • resources/read → read:resources:{{resource_id}}");
    println!("   • prompts/get    → get:prompts:{{prompt_name}}");
    println!("4. Checks permission using role system");
    println!("5. Allows/denies request based on result");

    println!("\n🔧 role-system Integration Benefits:");
    println!("-----------------------------------");
    println!("• Hierarchical roles (admin inherits user permissions)");
    println!("• Conditional permissions (business hours, trust levels)");
    println!("• Multiple subject types (User, Service, Device, Group)");
    println!("• Async support for database-backed role stores");
    println!("• Flexible permission format with wildcards");

    println!("\n📚 Example Usage in MoCoPr Server:");
    println!("----------------------------------");
    println!("```rust");
    println!("let server = ServerBuilder::new()");
    println!("    .name(\"Secure MCP Server\")");
    println!("    .add_middleware(RbacMiddleware::new(role_system))");
    println!("    .add_tool(SecureCalculatorTool::new())");
    println!("    .build()?;");
    println!("```");

    Ok(())
}
