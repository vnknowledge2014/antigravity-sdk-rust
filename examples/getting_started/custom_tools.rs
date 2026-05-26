// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Example demonstrating custom tools and stateful tools with ToolContext (Rust).
//!
//! This example shows:
//! 1. How to define a simple custom tool.
//! 2. How to define a stateful tool using ToolContext to maintain state across turns.
//!
//! To run:
//!   cargo run --example custom_tools

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::tools::{RegisteredTool, ToolRunner};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// Simple tool: looks up the SKU for a given fruit.
fn lookup_fruit_sku(
    args: Value,
) -> Pin<Box<dyn Future<Output = Result<Value, Box<dyn std::error::Error + Send + Sync>>> + Send>> {
    Box::pin(async move {
        let fruit_name = args
            .get("fruit_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let skus: HashMap<&str, &str> = HashMap::from([
            ("apple", "SKU-APP-123"),
            ("banana", "SKU-BAN-456"),
            ("orange", "SKU-ORA-789"),
        ]);

        let name = fruit_name.to_lowercase();
        let name_ref = if name.ends_with('s') && !skus.contains_key(name.as_str()) {
            &name[..name.len() - 1]
        } else {
            &name
        };

        let sku = skus.get(name_ref).unwrap_or(&"SKU-GEN-000");
        Ok(json!(format!(
            "SKU for {fruit_name} is {sku}. Order ID for restocking: ORD-{sku}-NEW"
        )))
    })
}

/// Stateful tool: records the count of fruits by SKU.
fn record_fruit(
    state: Arc<Mutex<HashMap<String, i64>>>,
) -> impl Fn(
    Value,
) -> Pin<
    Box<dyn Future<Output = Result<Value, Box<dyn std::error::Error + Send + Sync>>> + Send>,
> + Send
+ Sync {
    move |args: Value| {
        let state = state.clone();
        Box::pin(async move {
            let sku = args
                .get("sku")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN")
                .to_string();
            let count = args.get("count").and_then(|v| v.as_i64()).unwrap_or(0);

            let mut counts = state.lock().unwrap();
            let entry = counts.entry(sku.clone()).or_insert(0);
            *entry += count;
            let total = *entry;

            Ok(json!(format!(
                "Recorded {count} units for {sku}. Total count is now {total}."
            )))
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _config = LocalAgentConfig::default();

    // Create tool runner with custom tools
    let mut tool_runner = ToolRunner::new();

    // Register simple tool
    tool_runner.register(RegisteredTool {
        name: "lookup_fruit_sku".to_string(),
        description: "Looks up the SKU for a given fruit.".to_string(),
        schema: Some(json!({
            "type": "object",
            "properties": {
                "fruit_name": {"type": "string", "description": "The name of the fruit."}
            },
            "required": ["fruit_name"]
        })),
        handler: Box::new(lookup_fruit_sku),
    });

    // Register stateful tool
    let fruit_counts: Arc<Mutex<HashMap<String, i64>>> = Arc::new(Mutex::new(HashMap::new()));
    let handler = record_fruit(fruit_counts);
    tool_runner.register(RegisteredTool {
        name: "record_fruit".to_string(),
        description: "Records the count of fruits by SKU.".to_string(),
        schema: Some(json!({
            "type": "object",
            "properties": {
                "sku": {"type": "string", "description": "The SKU of the fruit."},
                "count": {"type": "integer", "description": "The number of fruits to record."}
            },
            "required": ["sku", "count"]
        })),
        handler: Box::new(handler),
    });

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    println!("  === Custom Tools Demo ===");

    // Test simple tool
    let prompt1 = "What is the SKU for apples? We need to order more.";
    println!("\n  User: {prompt1}");
    // TODO: agent.chat(prompt1) once fully wired
    println!("  Agent: [Response with SKU lookup result]");

    // Test stateful tool
    println!("\n  === Stateful Tool (Fruit Counter) Demo ===");
    let turns = [
        "I have 5 apples.",
        "And I just got 3 bananas.",
        "Oh, and another 2 apples.",
    ];
    for user_input in turns {
        println!("\n  User: {user_input}");
        println!("  Agent: [Response with running total]");
    }

    agent.stop().await?;
    Ok(())
}
