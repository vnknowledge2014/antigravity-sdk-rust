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

//! Example demonstrating streaming responses and thoughts in Google Antigravity SDK (Rust).
//!
//! To run:
//!   cargo run --example streaming

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _config = LocalAgentConfig::default();

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    let prompt = concat!(
        "Solve this riddle: I speak without a mouth and hear without ears. I ",
        "have no body, but I come alive with wind. What am I? Explain your ",
        "reasoning."
    );
    println!("  User: {prompt}\n");

    // TODO: Streaming iteration over response.thoughts and response tokens
    // will be available once the Response type is implemented.
    println!("  Agent (Streaming thoughts):");
    println!("  -------------------------------------------------------");
    println!("  [Thinking content would stream here...]");
    println!("  -------------------------------------------------------\n");

    println!("  Agent (Streaming final answer):");
    println!("  -------------------------------------------------------");
    println!("  An echo! It speaks without a mouth and hears without ears.");
    println!("  -------------------------------------------------------\n");

    agent.stop().await?;
    Ok(())
}
