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

//! Example demonstrating subagents in Google Antigravity SDK (Rust).
//!
//! Shows how an agent can spawn a subagent to delegate a specific task.
//!
//! To run:
//!   cargo run --example subagents

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::hooks::{DecideHook, HookContext, InspectHook};
use antigravity_sdk::types::{
    BuiltinTools, CapabilitiesConfig, HookResult, ToolCall, ToolName, ToolResult,
};
use async_trait::async_trait;

struct LogPreToolHook;

#[async_trait]
impl DecideHook<ToolCall> for LogPreToolHook {
    async fn run(&self, _ctx: &mut HookContext, data: &ToolCall) -> HookResult {
        if data.name == ToolName::Builtin(BuiltinTools::StartSubagent) {
            println!("\n  --- 🤖 [Hook] Spawning Subagent ---");
            println!("  Arguments: {:?}\n", data.args);
        } else {
            println!("  - [Start]: {} (ID: {:?})", data.name, data.id);
        }
        HookResult::allowed()
    }
}

struct LogPostToolHook;

#[async_trait]
impl InspectHook<ToolResult> for LogPostToolHook {
    async fn run(&self, _ctx: &mut HookContext, data: &ToolResult) {
        if data.name == ToolName::Builtin(BuiltinTools::StartSubagent) {
            println!("\n  --- 🤖 [Hook] Subagent Finished ---");
            println!("  Result: {:?}\n", data.result);
        } else {
            println!("  - [Done]: {} (ID: {:?}) ✅", data.name, data.id);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _config = LocalAgentConfig {
        capabilities: CapabilitiesConfig {
            enable_subagents: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    let prompt = concat!(
        "Use a subagent to research the Google Antigravity SDK examples in the parent ",
        "directory. Delegate the task of listing and reading the files to the ",
        "subagent, and then generate a lesson plan for me to learn more based ",
        "on its findings."
    );
    println!("  User: {prompt}");
    println!("\n  Agent: [Lesson plan based on subagent research]");

    agent.stop().await?;
    Ok(())
}
