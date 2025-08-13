#![allow(dead_code)]

use anyhow::Result;
use mocopr_core::{PromptGenerator, ResourceReader, ToolExecutor};
use mocopr_macros::{Prompt, Resource, Tool};
use mocopr_server::prelude::*;
use serde_json::{Value, json};
use std::collections::HashMap;
use tracing::info;

/// A resource that provides mathematical constants and formulas
#[derive(Resource)]
#[resource(
    name = "math_constants",
    description = "Mathematical constants and formulas"
)]
struct MathConstantsResource;

impl MathConstantsResource {
    async fn get_constants(&self) -> Result<Value> {
        Ok(json!({
            "pi": std::f64::consts::PI,
            "e": std::f64::consts::E,
            "sqrt_2": std::f64::consts::SQRT_2,
            "ln_2": std::f64::consts::LN_2,
            "ln_10": std::f64::consts::LN_10,
            "formulas": {
                "circle_area": "π × r²",
                "circle_circumference": "2 × π × r",
                "sphere_volume": "(4/3) × π × r³",
                "compound_interest": "P × (1 + r/n)^(n×t)"
            }
        }))
    }
}

#[async_trait::async_trait]
impl ResourceReader for MathConstantsResource {
    async fn read_resource(&self) -> mocopr_core::Result<Vec<ResourceContent>> {
        let constants = self.get_constants().await.unwrap();
        let uri = url::Url::parse("resource://math_constants").unwrap();
        let content = vec![Content::Text(TextContent::new(constants.to_string()))];

        Ok(vec![ResourceContent::new(uri, content)])
    }
}

/// Basic arithmetic operations tool
#[derive(Tool)]
#[tool(
    name = "arithmetic",
    description = "Perform basic arithmetic operations (add, subtract, multiply, divide)"
)]
struct ArithmeticTool;

impl ArithmeticTool {
    async fn execute_impl(&self, args: Value) -> Result<Value> {
        let operation = args
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: operation"))?;

        let a = args
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: a"))?;

        let b = args
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: b"))?;

        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => {
                if b == 0.0 {
                    return Err(anyhow::anyhow!("Division by zero"));
                }
                a / b
            }
            _ => return Err(anyhow::anyhow!("Unknown operation: {}", operation)),
        };

        Ok(json!({
            "operation": operation,
            "operands": [a, b],
            "result": result
        }))
    }
}

#[async_trait::async_trait]
impl ToolExecutor for ArithmeticTool {
    async fn execute(
        &self,
        arguments: Option<serde_json::Value>,
    ) -> mocopr_core::Result<mocopr_core::types::ToolsCallResponse> {
        let args = arguments.unwrap_or_default();
        match self.execute_impl(args).await {
            Ok(result) => Ok(mocopr_core::types::ToolsCallResponse::success(vec![
                mocopr_core::types::Content::Text(mocopr_core::types::TextContent::new(
                    result.to_string(),
                )),
            ])),
            Err(e) => Ok(mocopr_core::types::ToolsCallResponse::error(vec![
                mocopr_core::types::Content::Text(mocopr_core::types::TextContent::new(
                    e.to_string(),
                )),
            ])),
        }
    }
}

/// Advanced mathematical functions tool
#[derive(Tool)]
#[tool(
    name = "math_functions",
    description = "Perform advanced mathematical functions (sin, cos, tan, log, sqrt, pow)"
)]
struct MathFunctionsTool;

impl MathFunctionsTool {
    async fn execute_impl(&self, args: Value) -> Result<Value> {
        let function = args
            .get("function")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: function"))?;

        let x = args
            .get("x")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: x"))?;

        let result = match function {
            "sin" => x.sin(),
            "cos" => x.cos(),
            "tan" => x.tan(),
            "asin" => {
                if !(-1.0..=1.0).contains(&x) {
                    return Err(anyhow::anyhow!("asin input must be between -1 and 1"));
                }
                x.asin()
            }
            "acos" => {
                if !(-1.0..=1.0).contains(&x) {
                    return Err(anyhow::anyhow!("acos input must be between -1 and 1"));
                }
                x.acos()
            }
            "atan" => x.atan(),
            "log" => {
                if x <= 0.0 {
                    return Err(anyhow::anyhow!("log input must be positive"));
                }
                x.ln()
            }
            "log10" => {
                if x <= 0.0 {
                    return Err(anyhow::anyhow!("log10 input must be positive"));
                }
                x.log10()
            }
            "sqrt" => {
                if x < 0.0 {
                    return Err(anyhow::anyhow!("sqrt input must be non-negative"));
                }
                x.sqrt()
            }
            "exp" => x.exp(),
            "abs" => x.abs(),
            "floor" => x.floor(),
            "ceil" => x.ceil(),
            "round" => x.round(),
            _ => {
                // Handle power function with base parameter
                if function == "pow" {
                    let base = args.get("base").and_then(|v| v.as_f64()).ok_or_else(|| {
                        anyhow::anyhow!("Missing required parameter: base for pow function")
                    })?;
                    base.powf(x)
                } else {
                    return Err(anyhow::anyhow!("Unknown function: {}", function));
                }
            }
        };

        let mut response = json!({
            "function": function,
            "input": x,
            "result": result
        });

        // Add base parameter for power function in response
        if function == "pow"
            && let Some(base) = args.get("base")
        {
            response["base"] = base.clone();
        }

        Ok(response)
    }
}

#[async_trait::async_trait]
impl ToolExecutor for MathFunctionsTool {
    async fn execute(
        &self,
        arguments: Option<serde_json::Value>,
    ) -> mocopr_core::Result<mocopr_core::types::ToolsCallResponse> {
        let args = arguments.unwrap_or_default();
        match self.execute_impl(args).await {
            Ok(result) => Ok(mocopr_core::types::ToolsCallResponse::success(vec![
                mocopr_core::types::Content::Text(mocopr_core::types::TextContent::new(
                    result.to_string(),
                )),
            ])),
            Err(e) => Ok(mocopr_core::types::ToolsCallResponse::error(vec![
                mocopr_core::types::Content::Text(mocopr_core::types::TextContent::new(
                    e.to_string(),
                )),
            ])),
        }
    }
}

/// Statistical calculations tool
#[derive(Tool)]
#[tool(
    name = "statistics",
    description = "Calculate statistical measures (mean, median, mode, std_dev) for a dataset"
)]
struct StatisticsTool;

impl StatisticsTool {
    async fn execute_impl(&self, args: Value) -> Result<Value> {
        let data = args
            .get("data")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: data (array)"))?;

        let numbers: Result<Vec<f64>, _> = data
            .iter()
            .map(|v| {
                v.as_f64()
                    .ok_or_else(|| anyhow::anyhow!("All data elements must be numbers"))
            })
            .collect();
        let numbers = numbers?;

        if numbers.is_empty() {
            return Err(anyhow::anyhow!("Data array cannot be empty"));
        }

        // Mean
        let mean = numbers.iter().sum::<f64>() / numbers.len() as f64;

        // Median
        let mut sorted = numbers.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = if sorted.len() % 2 == 0 {
            (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
        } else {
            sorted[sorted.len() / 2]
        };

        // Mode (most frequent value)
        let mut frequency = HashMap::new();
        for &num in &numbers {
            *frequency.entry(num.to_bits()).or_insert(0) += 1;
        }
        let mode = frequency
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(bits, _)| f64::from_bits(*bits));

        // Standard deviation
        let variance =
            numbers.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / numbers.len() as f64;
        let std_dev = variance.sqrt();

        // Min and Max
        let min = sorted.first().copied().unwrap();
        let max = sorted.last().copied().unwrap();

        Ok(json!({
            "data": numbers,
            "count": numbers.len(),
            "mean": mean,
            "median": median,
            "mode": mode,
            "std_dev": std_dev,
            "variance": variance,
            "min": min,
            "max": max,
            "range": max - min
        }))
    }
}

#[async_trait::async_trait]
impl ToolExecutor for StatisticsTool {
    async fn execute(
        &self,
        arguments: Option<serde_json::Value>,
    ) -> mocopr_core::Result<mocopr_core::types::ToolsCallResponse> {
        let args = arguments.unwrap_or_default();
        match self.execute_impl(args).await {
            Ok(result) => Ok(mocopr_core::types::ToolsCallResponse::success(vec![
                mocopr_core::types::Content::Text(mocopr_core::types::TextContent::new(
                    result.to_string(),
                )),
            ])),
            Err(e) => Ok(mocopr_core::types::ToolsCallResponse::error(vec![
                mocopr_core::types::Content::Text(mocopr_core::types::TextContent::new(
                    e.to_string(),
                )),
            ])),
        }
    }
}

/// A prompt to suggest mathematical operations based on input
#[derive(Prompt)]
#[prompt(
    name = "math_assistant",
    description = "Provide mathematical assistance and suggestions"
)]
struct MathAssistantPrompt;

impl MathAssistantPrompt {
    async fn execute_impl(&self, args: Option<Value>) -> Result<String> {
        let problem = args
            .as_ref()
            .and_then(|v| v.get("problem"))
            .and_then(|v| v.as_str())
            .unwrap_or("general");

        let response = match problem {
            "geometry" => {
                "For geometry problems, I can help with:\n\
                - Circle calculations: Use math_constants resource for π, then arithmetic for area (π×r²) and circumference (2×π×r)\n\
                - Triangle calculations: Use arithmetic and math_functions (sqrt for Pythagorean theorem)\n\
                - Sphere volume: Use the formula from math_constants resource\n\
                \n\
                Example: To find circle area with radius 5:\n\
                1. Get π from math_constants\n\
                2. Use math_functions to calculate 5² (pow function)\n\
                3. Use arithmetic to multiply π × 25"
            }
            "trigonometry" => {
                "For trigonometry problems:\n\
                - Use math_functions tool with sin, cos, tan for basic ratios\n\
                - Use asin, acos, atan for inverse functions\n\
                - Remember: inputs for sin, cos, tan should be in radians\n\
                - To convert degrees to radians: multiply by π/180\n\
                \n\
                Example: sin(30°)\n\
                1. Convert 30° to radians: 30 × π/180 ≈ 0.5236\n\
                2. Use math_functions with function='sin' and x=0.5236"
            }
            "statistics" => {
                "For statistical analysis:\n\
                - Use the statistics tool with an array of data\n\
                - It calculates mean, median, mode, standard deviation, variance, min, max, and range\n\
                - Great for analyzing datasets and understanding data distribution\n\
                \n\
                Example: Analyze test scores [85, 92, 78, 96, 88, 82, 95]\n\
                Use statistics tool with data=[85, 92, 78, 96, 88, 82, 95]"
            }
            "calculus" => {
                "For calculus-related problems:\n\
                - Use math_functions for exponential (exp) and logarithmic (log, log10) functions\n\
                - Power functions available with pow\n\
                - For derivatives and integrals, I can guide you through the process\n\
                \n\
                Note: This calculator doesn't do symbolic calculus, but can evaluate functions at specific points"
            }
            _ => {
                "I can help with various mathematical operations:\n\
                \n\
                🔢 Basic Arithmetic: Use 'arithmetic' tool for +, -, ×, ÷\n\
                📐 Advanced Functions: Use 'math_functions' for trig, logs, roots, powers\n\
                📊 Statistics: Use 'statistics' tool for data analysis\n\
                📏 Constants: Use 'math_constants' resource for π, e, and formulas\n\
                \n\
                What type of math problem are you working on?\n\
                - geometry: Area, volume, perimeter calculations\n\
                - trigonometry: Sin, cos, tan and their inverses\n\
                - statistics: Data analysis and statistical measures\n\
                - calculus: Exponential and logarithmic functions\n\
                \n\
                Just describe your problem and I'll suggest the best approach!"
            }
        };

        Ok(response.to_string())
    }
}

#[async_trait::async_trait]
impl PromptGenerator for MathAssistantPrompt {
    async fn generate_prompt(
        &self,
        arguments: Option<HashMap<String, String>>,
    ) -> mocopr_core::Result<mocopr_core::types::PromptsGetResponse> {
        let problem = arguments
            .as_ref()
            .and_then(|args| args.get("problem"))
            .map(|s| s.as_str())
            .unwrap_or("general");

        let args_value = arguments
            .clone()
            .map(|args| json!(args.into_iter().collect::<HashMap<_, _>>()));

        let response_text = self
            .execute_impl(args_value)
            .await
            .map_err(|e| mocopr_core::Error::Internal(e.to_string()))?;

        let message = mocopr_core::types::PromptMessage::user(response_text);

        Ok(mocopr_core::types::PromptsGetResponse {
            description: Some(format!("Mathematical assistance for: {}", problem)),
            messages: vec![message],
            meta: mocopr_core::types::ResponseMetadata { _meta: None },
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting MCP Calculator Server");

    // Create resources
    let math_constants = MathConstantsResource;

    // Create tools
    let arithmetic_tool = ArithmeticTool;
    let math_functions_tool = MathFunctionsTool;
    let statistics_tool = StatisticsTool;

    // Create prompts
    let math_assistant_prompt = MathAssistantPrompt;

    // Build and start the server
    let server = McpServer::builder()
        .with_info("Calculator Server", "1.0.0")
        .with_resources()
        .with_tools()
        .with_prompts()
        .with_resource(math_constants)
        .with_tool(arithmetic_tool)
        .with_tool(math_functions_tool)
        .with_tool(statistics_tool)
        .with_prompt(math_assistant_prompt)
        .build()?;

    info!("MCP Calculator Server ready. Capabilities:");
    info!("- Resources: math_constants (π, e, formulas)");
    info!("- Tools: arithmetic, math_functions, statistics");
    info!("- Prompts: math_assistant");

    // Run the server using stdio transport
    server.run_stdio().await?;

    Ok(())
}
