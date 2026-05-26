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

//! Example demonstrating fully autonomous shell agent (Rust).
//!
//! WARNING: This gives the agent unrestricted shell access.
//!
//! To run:
//!   cargo run --example autonomous_shell

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::hooks::policy;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("  === Autonomous Shell Demo ===");
    println!("  WARNING: Agent has unrestricted tool access!");

    // allow_all() removes all default policy restrictions
    let _allow_all_policy = policy::allow_all();

    let _config = LocalAgentConfig::default();

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    let prompt = "Check the system uptime and report the result.";
    println!("\n  User: {prompt}");
    println!("  Agent: [Running `uptime` command... system has been up for 42 days]");

    agent.stop().await?;
    Ok(())
}
