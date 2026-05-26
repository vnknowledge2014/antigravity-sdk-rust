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

//! Deep dive: Async chat with concurrent processing (Rust).
//!
//! Demonstrates running multiple agent conversations concurrently using
//! tokio's async task system.
//!
//! To run:
//!   cargo run --example async_chat

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;

async fn run_conversation(
    id: usize,
    prompt: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _config = LocalAgentConfig::default();
    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    println!("  [Conv {id}] User: {prompt}");
    // TODO: agent.chat(prompt)
    println!("  [Conv {id}] Agent: [Response]");

    agent.stop().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("  === Async Chat Demo ===\n");

    let prompts = vec![
        "What is the capital of France?",
        "Explain quantum computing in one sentence.",
        "Write a haiku about async programming.",
    ];

    // Run conversations concurrently
    let handles: Vec<_> = prompts
        .into_iter()
        .enumerate()
        .map(|(i, prompt)| {
            let p = prompt.to_string();
            tokio::spawn(async move {
                if let Err(e) = run_conversation(i + 1, p).await {
                    eprintln!("  [Conv {}] Error: {e}", i + 1);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.await?;
    }

    println!("\n  All conversations completed!");
    Ok(())
}
