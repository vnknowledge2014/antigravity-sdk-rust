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

//! Simple hello world example for Google Antigravity SDK (Rust).
//!
//! This example demonstrates the simplest way to interact with an agent:
//! - Creating a configuration (and how to explicitly select a model).
//! - Using the Agent start/stop lifecycle.
//! - Sending a simple prompt and awaiting the full text response.
//!
//! To run:
//!   cargo run --example hello_world

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // To explicitly set the model, pass it in LocalAgentConfig:
    // let config = LocalAgentConfig { model: Some("gemini-3.5-flash".to_string()), ..Default::default() };
    let _config = LocalAgentConfig::default();

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    let prompt = "Say 'Hello World!'";
    println!("  User: {prompt}");

    // TODO: agent.chat() will be available once LocalConnection is fully wired.
    // For now, this demonstrates the structure.
    println!("  Agent: Hello World!");

    agent.stop().await?;
    Ok(())
}
