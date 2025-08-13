//! Prompt macro implementations
//!
//! This module provides derive macros for creating MCP prompts. The macros generate
//! the necessary trait implementations while requiring users to implement the actual
//! prompt logic through the `PromptGenerator` trait defined in mocopr_core.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, ItemFn, Meta, Result};

pub fn derive_prompt_impl(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;

    // Extract prompt attributes
    let mut prompt_name = name.to_string();
    let mut prompt_description = "Auto-generated prompt".to_string();

    for attr in &input.attrs {
        if attr.path().is_ident("prompt") {
            // Parse prompt attributes using proper AST parsing
            if let Ok(meta) = attr.parse_args::<syn::Meta>()
                && let syn::Meta::List(meta_list) = meta
            {
                let _ = meta_list.parse_nested_meta(|meta| {
                    if meta.path.is_ident("name") {
                        let value = meta.value()?;
                        let lit_str: syn::LitStr = value.parse()?;
                        prompt_name = lit_str.value();
                        Ok(())
                    } else if meta.path.is_ident("description") {
                        let value = meta.value()?;
                        let lit_str: syn::LitStr = value.parse()?;
                        prompt_description = lit_str.value();
                        Ok(())
                    } else {
                        Err(meta.error("unsupported prompt attribute"))
                    }
                });
            }
        }
    }

    let prompt_name = syn::LitStr::new(&prompt_name, name.span());
    let prompt_description = syn::LitStr::new(&prompt_description, name.span());

    let expanded = quote! {
        // Generate a compile-time reminder that PromptGenerator must be implemented
        const _: () = {
            fn assert_prompt_generator<T: mocopr_core::PromptGenerator>() {}
            fn assert_impl() {
                assert_prompt_generator::<#name>();
            }
        };

        #[async_trait::async_trait]
        impl mocopr_server::handlers::PromptHandler for #name {
            async fn prompt(&self) -> mocopr_core::types::Prompt {
                mocopr_core::types::Prompt::new(
                    #prompt_name
                ).with_description(#prompt_description)
            }

            async fn generate(&self, arguments: Option<std::collections::HashMap<String, String>>) -> mocopr_core::Result<mocopr_core::types::PromptsGetResponse> {
                self.generate_prompt(arguments).await
            }
        }
    };

    Ok(expanded)
}

pub fn mcp_prompt_impl(args: Meta, input: ItemFn) -> Result<TokenStream> {
    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_block = &input.block;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;

    // Extract prompt name and description from attributes
    let mut prompt_name = fn_name.to_string();
    let mut prompt_description = "Auto-generated prompt".to_string();

    // Simple parsing - in a real implementation you'd want more robust parsing
    let args_str = quote! { #args }.to_string();

    if args_str.contains("name =")
        && let Some(start) = args_str.find("name = \"")
    {
        let start = start + 8; // length of "name = \""
        if let Some(end) = args_str[start..].find('"') {
            prompt_name = args_str[start..start + end].to_string();
        }
    }

    if args_str.contains("description =")
        && let Some(start) = args_str.find("description = \"")
    {
        let start = start + 15; // length of "description = \""
        if let Some(end) = args_str[start..].find('"') {
            prompt_description = args_str[start..start + end].to_string();
        }
    }

    let struct_name = syn::Ident::new(&format!("{}Prompt", fn_name), fn_name.span());

    let expanded = quote! {
        #fn_vis struct #struct_name;

        impl #struct_name {
            #fn_vis async fn #fn_name(#fn_inputs) #fn_output #fn_block
        }

        #[async_trait::async_trait]
        impl mocopr_server::handlers::PromptHandler for #struct_name {
            async fn prompt(&self) -> mocopr_core::types::Prompt {
                mocopr_core::types::Prompt::new(
                    #prompt_name
                ).with_description(#prompt_description)
            }

            async fn generate(&self, arguments: Option<std::collections::HashMap<String, String>>) -> mocopr_core::Result<mocopr_core::types::PromptsGetResponse> {
                // Call the generated function with proper error handling
                match Self::#fn_name(arguments).await {
                    Ok(response) => Ok(response),
                    Err(e) => Err(mocopr_core::Error::operation_failed(
                        format!("Prompt generation failed: {}", e)
                    ))
                }
            }
        }
    };

    Ok(expanded)
}
