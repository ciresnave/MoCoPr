//! Resource macro implementations
//!
//! This module provides derive macros for creating MCP resources. The macros generate
//! the necessary trait implementations while requiring users to implement the actual
//! resource logic through the `ResourceReader` trait defined in mocopr_core.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, ItemStruct, Meta, Result};

pub fn derive_resource_impl(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;

    // Extract resource attributes
    let mut resource_uri = "resource://default".to_string();
    let mut resource_name = name.to_string();
    let mut resource_description = "Auto-generated resource".to_string();

    for attr in &input.attrs {
        if attr.path().is_ident("resource") {
            // Parse resource attributes using proper AST parsing
            if let Ok(meta) = attr.parse_args::<syn::Meta>()
                && let syn::Meta::List(meta_list) = meta
            {
                let _ = meta_list.parse_nested_meta(|meta| {
                    if meta.path.is_ident("uri") {
                        let value = meta.value()?;
                        let lit_str: syn::LitStr = value.parse()?;
                        resource_uri = lit_str.value();
                        Ok(())
                    } else if meta.path.is_ident("name") {
                        let value = meta.value()?;
                        let lit_str: syn::LitStr = value.parse()?;
                        resource_name = lit_str.value();
                        Ok(())
                    } else if meta.path.is_ident("description") {
                        let value = meta.value()?;
                        let lit_str: syn::LitStr = value.parse()?;
                        resource_description = lit_str.value();
                        Ok(())
                    } else {
                        Err(meta.error("unsupported resource attribute"))
                    }
                });
            }
        }
    }

    let resource_uri = syn::LitStr::new(&resource_uri, name.span());
    let resource_name = syn::LitStr::new(&resource_name, name.span());
    let resource_description = syn::LitStr::new(&resource_description, name.span());

    let expanded = quote! {
        // Generate a compile-time reminder that ResourceReader must be implemented
        const _: () = {
            fn assert_resource_reader<T: mocopr_core::ResourceReader>() {}
            fn assert_impl() {
                assert_resource_reader::<#name>();
            }
        };

        #[async_trait::async_trait]
        impl mocopr_server::handlers::ResourceHandler for #name {
            async fn resource(&self) -> mocopr_core::types::Resource {
                mocopr_core::types::Resource::new(
                    url::Url::parse(#resource_uri).unwrap(),
                    #resource_name
                ).with_description(#resource_description)
            }

            async fn read(&self) -> mocopr_core::Result<Vec<mocopr_core::types::ResourceContent>> {
                self.read_resource().await
            }
        }
    };

    Ok(expanded)
}

pub fn mcp_resource_impl(args: Meta, input: ItemStruct) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let struct_vis = &input.vis;
    let struct_fields = &input.fields;

    // Extract resource URI, name, and description from attributes
    let mut resource_uri = "resource://default".to_string();
    let mut resource_name = struct_name.to_string();
    let mut resource_description = "Auto-generated resource".to_string();

    // Simple parsing - in a real implementation you'd want more robust parsing
    let args_str = quote! { #args }.to_string();

    if args_str.contains("uri =")
        && let Some(start) = args_str.find("uri = \"")
    {
        let start = start + 7; // length of "uri = \""
        if let Some(end) = args_str[start..].find('"') {
            resource_uri = args_str[start..start + end].to_string();
        }
    }

    if args_str.contains("name =")
        && let Some(start) = args_str.find("name = \"")
    {
        let start = start + 8; // length of "name = \""
        if let Some(end) = args_str[start..].find('"') {
            resource_name = args_str[start..start + end].to_string();
        }
    }

    if args_str.contains("description =")
        && let Some(start) = args_str.find("description = \"")
    {
        let start = start + 15; // length of "description = \""
        if let Some(end) = args_str[start..].find('"') {
            resource_description = args_str[start..start + end].to_string();
        }
    }

    let expanded = quote! {
        #struct_vis struct #struct_name #struct_fields

        #[async_trait::async_trait]
        impl mocopr_server::handlers::ResourceHandler for #struct_name {
            async fn resource(&self) -> mocopr_core::types::Resource {
                mocopr_core::types::Resource::new(
                    url::Url::parse(#resource_uri).unwrap(),
                    #resource_name
                ).with_description(#resource_description)
            }

            async fn read(&self) -> mocopr_core::Result<Vec<mocopr_core::types::ResourceContent>> {
                // Call the generated function with proper error handling
                match Self::#struct_name().await {
                    Ok(content) => Ok(content),
                    Err(e) => Err(mocopr_core::Error::resource_error(
                        format!("Resource reading failed: {}", e)
                    ))
                }
            }
        }
    };

    Ok(expanded)
}
