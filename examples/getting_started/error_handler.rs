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

//! Example demonstrating on_tool_error hook for error handling (Rust).
//!
//! To run:
//!   cargo run --example error_handler

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("  === Error Handler Demo ===");

    let _config = LocalAgentConfig::default();

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    // Simulate a tool error
    println!("  [Hook] on_tool_error fired: RuntimeError: This tool is broken!");
    println!("  Agent: I encountered an error with that tool. Let me try another approach.");

    agent.stop().await?;
    Ok(())
}
