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

//! Deep dive: Interactive CLI agent loop (Rust).
//!
//! Demonstrates building a REPL-style agent that reads user input from stdin,
//! sends it to the agent, and streams responses back.
//!
//! To run:
//!   cargo run --example interactive_cli

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("  === Interactive CLI Agent ===");
    println!("  Type your message and press Enter. Type 'quit' to exit.\n");

    let _config = LocalAgentConfig::default();

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    let stdin = io::stdin();
    loop {
        print!("  You> ");
        io::stdout().flush()?;

        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }
        if input == "quit" || input == "exit" {
            break;
        }

        // TODO: Send to agent and stream response
        println!("  Agent> [Response to: {input}]");
    }

    println!("\n  Goodbye!");
    agent.stop().await?;
    Ok(())
}
