# API Stability Guarantees

This document outlines the API stability guarantees for the MoCoPr (More Copper) project and its commitment to semantic versioning.

## Versioning Strategy

MoCoPr follows [Semantic Versioning 2.0.0](https://semver.org/) with the following specific guarantees:

### Version Format: MAJOR.MINOR.PATCH

- **MAJOR**: Incompatible API changes
- **MINOR**: New functionality, backward compatible
- **PATCH**: Bug fixes, backward compatible

### Current Status: 0.1.0 (Pre-1.0 Development)

**⚠️ IMPORTANT**: While in pre-1.0 development (0.x.y), breaking changes may occur in minor versions. We will clearly document all breaking changes in the changelog.

## API Stability Levels

### Level 1: Stable API (Post-1.0)
- **Public API**: All public functions, types, and traits are stable
- **Semantic Versioning**: Strictly enforced
- **Breaking Changes**: Only in major version increments
- **Deprecation Policy**: 2 major versions minimum before removal

### Level 2: Experimental API (Pre-1.0)
- **Current Status**: 0.1.0
- **Breaking Changes**: May occur in minor versions with clear documentation
- **Migration Path**: Always provided for breaking changes
- **Stability Timeline**: Target 1.0 stable release by Q4 2025

## Stable API Components

### Core Types (`mocopr-core`)
```rust
// Stable API surface
pub struct Message { /* ... */ }
pub struct Request { /* ... */ }
pub struct Response { /* ... */ }
pub struct Notification { /* ... */ }

// Stable error types
pub enum McpError { /* ... */ }
pub type Result<T> = std::result::Result<T, McpError>;
```

### Server API (`mocopr-server`)
```rust
// Stable builder pattern
pub struct ServerBuilder { /* ... */ }
impl ServerBuilder {
    pub fn new() -> Self { /* ... */ }
    pub fn name(self, name: &str) -> Self { /* ... */ }
    pub fn version(self, version: &str) -> Self { /* ... */ }
    pub fn add_tool<T: Tool>(self, tool: T) -> Self { /* ... */ }
    pub fn build(self) -> Result<Server> { /* ... */ }
}

// Stable server traits
pub trait Tool: Send + Sync {
    async fn call(&self, params: Value) -> Result<Value>;
    fn parameters(&self) -> HashMap<String, ToolParameter>;
}
```

### Client API (`mocopr-client`)
```rust
// Stable client interface
pub struct Client { /* ... */ }
impl Client {
    pub async fn connect(transport: impl Transport) -> Result<Self> { /* ... */ }
    pub async fn call_tool(&self, name: &str, params: Value) -> Result<Value> { /* ... */ }
    pub async fn list_tools(&self) -> Result<Vec<Tool>> { /* ... */ }
}
```

## Deprecation Policy

### Pre-1.0 (Current)
- **Notice Period**: 1 minor version minimum
- **Documentation**: Deprecated items clearly marked
- **Migration Guide**: Provided for all deprecations
- **Removal Timeline**: Next major version

### Post-1.0
- **Notice Period**: 2 major versions minimum
- **Documentation**: Comprehensive deprecation warnings
- **Migration Guide**: Detailed migration paths
- **Removal Timeline**: Only in major version increments

## Breaking Change Guidelines

### What Constitutes a Breaking Change

1. **Function Signatures**: Changing parameter types, return types, or function names
2. **Public Types**: Modifying public struct fields or enum variants
3. **Trait Changes**: Adding required methods or changing existing ones
4. **Error Types**: Changing error variants or error semantics
5. **Behavior Changes**: Altering documented behavior

### What Does NOT Constitute a Breaking Change

1. **Bug Fixes**: Correcting incorrect behavior
2. **Performance Improvements**: Optimizations that don't change API
3. **Internal Changes**: Private implementation details
4. **Documentation**: Updates and clarifications
5. **New Features**: Adding new optional functionality

## Compatibility Testing

### Automated Compatibility Checks
- **API Diff**: Automated detection of breaking changes
- **Integration Tests**: Comprehensive test suite for API compatibility
- **Example Validation**: All examples tested against API changes

### Manual Review Process
- **Breaking Change Review**: All breaking changes require maintainer approval
- **Migration Path Validation**: Ensure migration paths are feasible
- **Documentation Review**: Verify all changes are properly documented

## Migration Support

### Pre-1.0 Migration Support
- **Changelog**: Detailed breaking change documentation
- **Migration Guide**: Step-by-step migration instructions
- **Example Updates**: Updated examples for new API
- **Community Support**: GitHub discussions for migration help

### Post-1.0 Migration Support
- **Deprecation Warnings**: Compile-time warnings for deprecated APIs
- **Migration Tools**: Automated migration tools where possible
- **LTS Support**: Extended support for critical versions
- **Professional Support**: Available for enterprise users

## API Evolution Strategy

### Phase 1: Pre-1.0 (Current)
- **Goal**: Stabilize core API surface
- **Timeline**: 6-12 months
- **Focus**: User feedback integration, API refinement
- **Breaking Changes**: Allowed with documentation

### Phase 2: 1.0 Stable Release
- **Goal**: Stable, production-ready API
- **Timeline**: Q4 2025
- **Focus**: Performance optimization, comprehensive testing
- **Breaking Changes**: Major version only

### Phase 3: Post-1.0 Evolution
- **Goal**: Continuous improvement with stability
- **Timeline**: Ongoing
- **Focus**: New features, ecosystem growth
- **Breaking Changes**: Rare, major version only

## Stability Guarantees by Crate

### `mocopr-core`
- **Stability Level**: Highest priority for 1.0 stability
- **Change Policy**: Most conservative approach
- **Dependencies**: Minimal, well-established crates only

### `mocopr-server`
- **Stability Level**: High priority for 1.0 stability
- **Change Policy**: Builder pattern stability guaranteed
- **Dependencies**: Stable core + server ecosystem

### `mocopr-client`
- **Stability Level**: High priority for 1.0 stability
- **Change Policy**: Connection API stability guaranteed
- **Dependencies**: Stable core + client ecosystem

### `mocopr-macros`
- **Stability Level**: Medium priority
- **Change Policy**: Macro syntax stability post-1.0
- **Dependencies**: Proc-macro ecosystem

### `mocopr-rbac`
- **Stability Level**: Experimental
- **Change Policy**: May have breaking changes pre-1.0
- **Dependencies**: Security-focused, regularly updated

## Commitment to Users

### Our Promise
1. **Clear Communication**: All changes clearly documented
2. **Migration Support**: Always provide migration paths
3. **Stability**: Respect semantic versioning commitments
4. **Feedback**: Community input valued in API decisions

### User Expectations
1. **Read Changelogs**: Always review changelog before upgrading
2. **Test Upgrades**: Test in development before production
3. **Provide Feedback**: Report issues and suggest improvements
4. **Migration Planning**: Plan for breaking changes in major versions

## Contact and Support

- **GitHub Issues**: For bug reports and feature requests
- **GitHub Discussions**: For API design discussions
- **Email**: ciresnave@gmail.com for critical stability concerns
- **Documentation**: https://docs.rs/mocopr for API documentation

## Changelog Policy

All releases include:
- **Added**: New features
- **Changed**: Changes in existing functionality
- **Deprecated**: Soon-to-be removed features
- **Removed**: Features removed in this version
- **Fixed**: Bug fixes
- **Security**: Security vulnerability fixes

---

*This document is updated with each release. Last updated: July 17, 2025*
