//! Tool macro implementations
//!
//! This module provides derive macros for MCP tools. The macros generate
//! the necessary trait implementations while requiring users to implement the actual
//! tool logic through the `ToolExecutor` trait defined in mocopr_core.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, ItemFn, Meta, Result};

/// Derive macro implementation for Tool trait
///
/// This generates a `ToolHandler` implementation that requires the user to also
/// implement the `ToolExecutor` trait with the actual tool logic.
pub fn derive_tool_impl(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;

    // Extract tool attributes using proper AST parsing
    let mut tool_name = None;
    let mut tool_description = None;

    for attr in &input.attrs {
        if attr.path().is_ident("tool") {
            // Use proper AST parsing instead of string manipulation
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let lit_str: syn::LitStr = value.parse()?;
                    let name_val = lit_str.value();
                    if name_val.is_empty() {
                        return Err(meta.error("tool name cannot be empty"));
                    }
                    tool_name = Some(name_val);
                    Ok(())
                } else if meta.path.is_ident("description") {
                    let value = meta.value()?;
                    let lit_str: syn::LitStr = value.parse()?;
                    tool_description = Some(lit_str.value());
                    Ok(())
                } else {
                    let path = meta
                        .path
                        .get_ident()
                        .map(|i| i.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    Err(meta.error(format!("unsupported tool attribute: `{}`", path)))
                }
            })?;
        }
    }

    let default_name = name.to_string().to_lowercase();
    let tool_name_str = tool_name.as_deref().unwrap_or(&default_name);
    let tool_description_str = tool_description.as_deref().unwrap_or("Auto-generated tool");

    let expanded = quote! {
        #[::async_trait::async_trait]
        impl ::mocopr_server::ToolHandler for #name {
            async fn tool(&self) -> ::mocopr_core::types::Tool {
                ::mocopr_core::types::Tool::new(
                    #tool_name_str,
                    ::serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "required": []
                    })
                ).with_description(#tool_description_str)
            }

            async fn call(
                &self,
                arguments: Option<::serde_json::Value>,
            ) -> ::mocopr_core::Result<::mocopr_core::types::ToolsCallResponse> {
                // Delegate to the ToolExecutor trait which users must implement
                match self.execute(arguments).await {
                    Ok(response) => Ok(response),
                    Err(e) => Err(::mocopr_core::Error::Internal(e.to_string()))
                }
            }
        }

        // Compile-time assertion to ensure ToolExecutor is implemented
        const _: fn() = || {
            fn assert_impl<T: ::mocopr_core::ToolExecutor>() {}
            assert_impl::<#name>();
        };
    };

    Ok(expanded)
}

/// Function-based tool macro implementation
///
/// This generates a struct and trait implementations for a function-based tool.
pub fn mcp_tool_impl(args: Meta, input: ItemFn) -> Result<TokenStream> {
    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_block = &input.block;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;

    // Extract tool name and description from attributes using proper AST parsing
    let mut tool_name = fn_name.to_string();
    let mut tool_description = "Auto-generated tool".to_string();

    // Parse attributes using syn's built-in attribute parsing
    if let syn::Meta::List(meta_list) = args {
        // Parse nested attributes directly
        let _ = meta_list.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                let value = meta.value()?;
                let lit_str: syn::LitStr = value.parse()?;
                let name_val = lit_str.value();
                if name_val.is_empty() {
                    return Err(meta.error("tool name cannot be empty"));
                }
                tool_name = name_val;
                Ok(())
            } else if meta.path.is_ident("description") {
                let value = meta.value()?;
                let lit_str: syn::LitStr = value.parse()?;
                tool_description = lit_str.value();
                Ok(())
            } else {
                let path = meta
                    .path
                    .get_ident()
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                Err(meta.error(format!("unsupported tool attribute: `{}`", path)))
            }
        });
    }

    let struct_name = syn::Ident::new(&format!("{}Tool", fn_name), fn_name.span());

    let expanded = quote! {
        #fn_vis struct #struct_name;

        impl #struct_name {
            #fn_vis async fn #fn_name(#fn_inputs) #fn_output #fn_block
        }

        #[::async_trait::async_trait]
        impl ::mocopr_server::ToolHandler for #struct_name {
            async fn tool(&self) -> ::mocopr_core::types::Tool {
                ::mocopr_core::types::Tool::new(
                    #tool_name,
                    ::serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "required": []
                    })
                ).with_description(#tool_description)
            }

            async fn call(
                &self,
                arguments: Option<::serde_json::Value>,
            ) -> ::mocopr_core::Result<::mocopr_core::types::ToolsCallResponse> {
                // Delegate to the ToolExecutor trait which users must implement
                match self.execute(arguments).await {
                    Ok(response) => Ok(response),
                    Err(e) => Err(::mocopr_core::Error::Internal(e.to_string()))
                }
            }
        }

        #[::async_trait::async_trait]
        impl ::mocopr_core::ToolExecutor for #struct_name {
            async fn execute(
                &self,
                arguments: Option<::serde_json::Value>,
            ) -> ::anyhow::Result<::mocopr_core::types::ToolsCallResponse> {
                // Convert function call to tool response
                let result = Self::#fn_name(arguments).await?;
                Ok(result)
            }
        }

        // Compile-time assertion to ensure function signature compatibility
        const _: fn() = || {
            fn assert_impl<T: ::mocopr_core::ToolExecutor>() {}
            assert_impl::<#struct_name>();
        };

        #input
    };

    Ok(expanded)
}
