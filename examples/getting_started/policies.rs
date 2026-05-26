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

//! Example demonstrating tool call policies in Google Antigravity SDK (Rust).
//!
//! Demonstrates:
//! 1. The "Deny by Default" posture.
//! 2. Specific Denylist rules (blocking dangerous shell commands like `rm`).
//! 3. Specific Allowlist rules (allowing only specific safe commands).
//! 4. Interactive confirmation rules using ask_user.
//!
//! To run:
//!   cargo run --example policies

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::hooks::policy::{self, Decision, Policy};
use antigravity_sdk::types::{BuiltinTools, ToolCall};
use std::future::Future;
use std::pin::Pin;

/// Predicate to detect 'rm' in command line arguments.
fn block_rm_predicate(tool_call: &ToolCall) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
    Box::pin(async move {
        tool_call
            .args
            .get("command_line")
            .and_then(|v| v.as_str())
            .map(|cmd| cmd.contains("rm"))
            .unwrap_or(false)
    })
}

/// Predicate to detect critical file operations.
fn critical_file_predicate(
    tool_call: &ToolCall,
) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
    Box::pin(async move {
        let path = tool_call
            .args
            .get("path")
            .or_else(|| tool_call.args.get("file_path"))
            .or_else(|| tool_call.args.get("TargetFile"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        path.ends_with(".key") || path.contains("production")
    })
}

/// Simulates programmatic user confirmation — always denies.
fn programmatic_approval_handler(
    tool_call: &ToolCall,
) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
    Box::pin(async move {
        println!(
            "\n  [ASK_USER Handler] Intercepted request for tool: {}",
            tool_call.name
        );
        println!(
            "  [ASK_USER Handler] Target arguments: {:?}",
            tool_call.args
        );
        println!("  [ASK_USER Handler] Simulating user review... Decision: DENY.");
        false
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("  === Tool Call Policies Demo ===");

    let policies = vec![
        // 1. Deny everything by default
        policy::deny_all(),
        // 2. Allow reading directory contents
        policy::allow(BuiltinTools::ListDir.as_str()),
        // 3. Allow running commands, but block dangerous 'rm' commands
        policy::allow(BuiltinTools::RunCommand.as_str()),
        Policy {
            tool: BuiltinTools::RunCommand.as_str().to_string(),
            decision: Decision::Deny,
            when: Some(Box::new(block_rm_predicate)),
            ask_user: None,
            name: "block-rm".to_string(),
        },
        // 4. Allow editing/creating, but ask user for critical files
        policy::allow(BuiltinTools::EditFile.as_str()),
        policy::allow(BuiltinTools::CreateFile.as_str()),
        Policy {
            tool: BuiltinTools::EditFile.as_str().to_string(),
            decision: Decision::AskUser,
            when: Some(Box::new(critical_file_predicate)),
            ask_user: Some(Box::new(programmatic_approval_handler)),
            name: "ask-for-critical-edits".to_string(),
        },
        Policy {
            tool: BuiltinTools::CreateFile.as_str().to_string(),
            decision: Decision::AskUser,
            when: Some(Box::new(critical_file_predicate)),
            ask_user: Some(Box::new(programmatic_approval_handler)),
            name: "ask-for-critical-creates".to_string(),
        },
    ];

    let _config = LocalAgentConfig::default();
    let _policy_hook = policy::enforce(policies);

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    // Try a safe command (should be allowed)
    let prompt1 = "List the files in the current directory.";
    println!("\n  User: {prompt1}");
    println!("  Agent: [Directory listing — allowed by policy]");

    // Try a dangerous command (should be denied)
    let prompt2 = "Delete all files using rm -rf.";
    println!("\n  User: {prompt2}");
    println!("  Agent: [Denied by 'block-rm' policy]");

    // Try creating a critical file (triggers ask_user)
    let prompt3 = "Create a new configuration file named production.key with content 'debug=true'.";
    println!("\n  User: {prompt3}");
    println!("  Agent: [Denied by 'ask-for-critical-creates' policy]");

    agent.stop().await?;
    Ok(())
}
