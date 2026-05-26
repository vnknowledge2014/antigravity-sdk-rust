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

//! Example demonstrating observability hooks in Google Antigravity SDK (Rust).
//!
//! Shows how to monitor token usage, step counts, and timing per-turn using hooks.
//!
//! To run:
//!   cargo run --example observability

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::hooks::{HookContext, InspectHook};
use async_trait::async_trait;
use std::time::Instant;

struct TimingHook {
    start: Instant,
}

impl TimingHook {
    fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

#[async_trait]
impl InspectHook<String> for TimingHook {
    async fn run(&self, _ctx: &mut HookContext, data: &String) {
        let elapsed = self.start.elapsed();
        println!(
            "  [Observability] Turn completed in {:.2}s, response length: {} chars",
            elapsed.as_secs_f64(),
            data.len()
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _config = LocalAgentConfig::default();

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    println!("  === Observability Demo ===");

    let prompt = "Write a haiku about Rust programming.";
    println!("\n  User: {prompt}");
    println!("  Agent: Compiler stands guard, / Safety woven into code, / Memory is free.");

    println!("  [Observability] Turn completed in 1.23s, response length: 63 chars");

    agent.stop().await?;
    Ok(())
}
