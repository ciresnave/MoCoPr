//! Procedural macros for MoCoPr
//!
//! This crate provides convenient macros for defining MCP tools, resources,
//! and other components with minimal boilerplate.

use proc_macro::TokenStream;
use syn::{DeriveInput, ItemFn, ItemStruct, parse_macro_input};

mod prompt;
mod resource;
mod tool;

/// Derive macro for automatically implementing ToolHandler
#[proc_macro_derive(Tool, attributes(tool))]
pub fn derive_tool(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    tool::derive_tool_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derive macro for automatically implementing ResourceHandler
#[proc_macro_derive(Resource, attributes(resource))]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    resource::derive_resource_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derive macro for automatically implementing PromptHandler
#[proc_macro_derive(Prompt, attributes(prompt))]
pub fn derive_prompt(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    prompt::derive_prompt_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Attribute macro for defining MCP tool functions
#[proc_macro_attribute]
pub fn mcp_tool(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as syn::Meta);
    let input = parse_macro_input!(input as ItemFn);
    tool::mcp_tool_impl(args, input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Attribute macro for defining MCP resource structs
#[proc_macro_attribute]
pub fn mcp_resource(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as syn::Meta);
    let input = parse_macro_input!(input as ItemStruct);
    resource::mcp_resource_impl(args, input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Attribute macro for defining MCP prompt functions
#[proc_macro_attribute]
pub fn mcp_prompt(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as syn::Meta);
    let input = parse_macro_input!(input as ItemFn);
    prompt::mcp_prompt_impl(args, input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
