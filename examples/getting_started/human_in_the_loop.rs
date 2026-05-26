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

//! Example demonstrating Human-in-the-Loop interaction in Google Antigravity SDK (Rust).
//!
//! Demonstrates how an agent can pause execution to ask the user for input
//! or clarification.
//!
//! To run:
//!   cargo run --example human_in_the_loop

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _config = LocalAgentConfig {
        system_instructions: Some(antigravity_sdk::types::SystemInstructions::Templated(
            antigravity_sdk::types::TemplatedSystemInstructions {
                identity: None,
                sections: vec![antigravity_sdk::types::SystemInstructionSection {
                    content: "When you need clarification or more information from the user \
                              to fulfill a request, you should use the `ask_question` tool \
                              to prompt them."
                        .to_string(),
                    title: "system_instructions".to_string(),
                }],
            },
        )),
        ..Default::default()
    };

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    let prompt = "I want to search for a file.";
    println!("  User: {prompt}");

    // TODO: Stream response and handle AskQuestion interaction
    println!("  Agent: What file are you looking for?");

    agent.stop().await?;
    Ok(())
}
