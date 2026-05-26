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

//! Deep dive: Host tool hooks (Rust).
//!
//! Demonstrates using pre/post tool call hooks specifically for host-side
//! (builtin) tools to add logging, timing, and audit trails.
//!
//! To run:
//!   cargo run --example host_tool_hooks

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::hooks::{DecideHook, HookContext, InspectHook};
use antigravity_sdk::types::{HookResult, ToolCall, ToolResult};
use async_trait::async_trait;

struct AuditPreToolHook;

#[async_trait]
impl DecideHook<ToolCall> for AuditPreToolHook {
    async fn run(&self, _ctx: &mut HookContext, data: &ToolCall) -> HookResult {
        println!(
            "  [Audit] Tool call: {} with args: {:?}",
            data.name, data.args
        );
        HookResult::allowed()
    }
}

struct AuditPostToolHook;

#[async_trait]
impl InspectHook<ToolResult> for AuditPostToolHook {
    async fn run(&self, _ctx: &mut HookContext, data: &ToolResult) {
        println!("  [Audit] Tool result: {} -> {:?}", data.name, data.result);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("  === Host Tool Hooks Demo ===\n");

    let _config = LocalAgentConfig::default();

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    println!("  User: List the files in the current directory.");
    println!("  [Audit] Tool call: list_directory with args: {{\"path\": \".\"}}");
    println!("  [Audit] Tool result: list_directory -> [file list]");
    println!("  Agent: Here are the files...\n");

    agent.stop().await?;
    Ok(())
}
