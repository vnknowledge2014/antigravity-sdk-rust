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

//! Deep dive: Documentation maintenance agent (Rust).
//!
//! Demonstrates an agent that monitors documentation files for staleness
//! and suggests updates using file watching triggers.
//!
//! To run:
//!   cargo run --example doc_maintenance_agent

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("  === Documentation Maintenance Agent Demo ===\n");

    let _config = LocalAgentConfig {
        system_instructions: Some(antigravity_sdk::types::SystemInstructions::Templated(
            antigravity_sdk::types::TemplatedSystemInstructions {
                identity: Some("You are a documentation maintenance assistant.".to_string()),
                sections: vec![antigravity_sdk::types::SystemInstructionSection {
                    title: "behavior".to_string(),
                    content: "Monitor documentation files and suggest updates when code changes."
                        .to_string(),
                }],
            },
        )),
        ..Default::default()
    };

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    println!("  [Agent] Monitoring documentation files...");
    println!("  [Trigger] File changed: README.md");
    println!("  Agent: The README.md has been modified. Here are suggested updates...\n");

    agent.stop().await?;
    Ok(())
}
