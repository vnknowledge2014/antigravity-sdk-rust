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

//! Deep dive: Round-based chat (Rust).
//!
//! Demonstrates a structured multi-turn conversation where each round has
//! a specific purpose (research, analyze, synthesize).
//!
//! To run:
//!   cargo run --example round_based_chat

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("  === Round-Based Chat Demo ===\n");

    let _config = LocalAgentConfig::default();

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    let rounds = [
        (
            "Round 1: Research",
            "Find information about Rust's ownership model.",
        ),
        (
            "Round 2: Analyze",
            "Compare Rust's ownership with C++ RAII.",
        ),
        (
            "Round 3: Synthesize",
            "Write a summary of the key differences.",
        ),
    ];

    for (round_name, prompt) in rounds {
        println!("  --- {round_name} ---");
        println!("  User: {prompt}");
        println!("  Agent: [Response for {round_name}]\n");
    }

    agent.stop().await?;
    Ok(())
}
