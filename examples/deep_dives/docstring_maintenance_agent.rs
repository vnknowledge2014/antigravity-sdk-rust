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

//! Deep dive: Docstring maintenance agent (Rust).
//!
//! Demonstrates an agent that reviews and updates docstrings/comments
//! in source code files.
//!
//! To run:
//!   cargo run --example docstring_maintenance_agent

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("  === Docstring Maintenance Agent Demo ===\n");

    let _config = LocalAgentConfig {
        system_instructions: Some(antigravity_sdk::types::SystemInstructions::Templated(
            antigravity_sdk::types::TemplatedSystemInstructions {
                identity: Some(
                    "You are a code documentation specialist. You review and improve \
                     doc comments in Rust source files."
                        .to_string(),
                ),
                sections: vec![],
            },
        )),
        ..Default::default()
    };

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    println!("  User: Review the docstrings in src/types.rs");
    println!("  Agent: I found 3 functions with missing or incomplete doc comments...\n");

    agent.stop().await?;
    Ok(())
}
