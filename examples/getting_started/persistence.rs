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

//! Example demonstrating session persistence in Google Antigravity SDK (Rust).
//!
//! Shows how to resume a previous agent conversation by specifying a
//! conversation ID and save directory.
//!
//! To run:
//!   cargo run --example persistence

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;
use tempfile::tempdir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let temp_dir_obj = tempdir()?;
    let save_dir = temp_dir_obj.path().to_string_lossy().to_string();

    // Session 1: Start a new conversation
    println!("  === Session 1: New Conversation ===");
    let _config1 = LocalAgentConfig {
        save_dir: Some(save_dir.clone()),
        ..Default::default()
    };

    let mut agent1 = Agent::new(Default::default());
    agent1.start().await?;

    println!("  User: Remember this: the secret code is 42.");
    println!("  Agent: Got it! I'll remember the secret code is 42.");

    let conversation_id = agent1
        .conversation_id()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "demo-conversation-id".to_string());
    println!("  Conversation ID: {conversation_id}");

    agent1.stop().await?;

    // Session 2: Resume the conversation
    println!("\n  === Session 2: Resumed Conversation ===");
    let _config2 = LocalAgentConfig {
        save_dir: Some(save_dir),
        conversation_id: Some(conversation_id),
        ..Default::default()
    };

    let mut agent2 = Agent::new(Default::default());
    agent2.start().await?;

    println!("  User: What was the secret code I told you earlier?");
    println!("  Agent: The secret code you told me was 42!");

    agent2.stop().await?;
    Ok(())
}
