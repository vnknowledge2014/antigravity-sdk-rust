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

//! Example: Using the Functional Core — Pure Pipeline Functions
//!
//! This example demonstrates how the SDK's pure functions in `src/core/`
//! can be used directly for validation, tool parsing, and pipeline composition.
//! All functions shown here are **pure** — no IO, no async, fully deterministic.

use antigravity_sdk::core::{agent_core, pipeline, step_core, tool_core};
use antigravity_sdk::types::UsageMetadata;

fn main() {
    println!("╔══════════════════════════════════════════╗");
    println!("║   Functional Core Demo — Pure Functions  ║");
    println!("╚══════════════════════════════════════════╝");
    println!();

    // ─────────────────────────────────────────────────────────────
    // 1. Agent Core: Safety Validation (pure function)
    // ─────────────────────────────────────────────────────────────
    println!("── Agent Core: Safety Validation ──");
    let active_tools = vec!["shell".to_string(), "read_file".to_string()];
    let has_mcp = false;
    let policy_count = 1;
    let has_hooks = true;

    match agent_core::validate_safety(&active_tools, has_mcp, policy_count, has_hooks) {
        Ok(()) => println!("  ✅ Safety check passed"),
        Err(e) => println!("  ❌ Safety check failed: {}", e),
    }

    let has_writes = agent_core::has_write_tools(&active_tools);
    println!("  Has write tools: {}", has_writes);
    println!();

    // ─────────────────────────────────────────────────────────────
    // 2. Tool Core: Parse Wire Tool Call (pure function)
    // ─────────────────────────────────────────────────────────────
    println!("── Tool Core: Parse Wire Tool Call ──");
    let tc = tool_core::parse_wire_tool_call(
        "call-123".to_string(),
        "get_weather".to_string(),
        r#"{"city": "Tokyo"}"#.to_string(),
    );
    match tc {
        Ok(tc) => println!("  ✅ Parsed: {} (id={})", tc.name, tc.id),
        Err(e) => println!("  ❌ Parse failed: {}", e),
    }

    // Denial result
    println!();
    println!("── Tool Core: Build Denial Result ──");
    let tc2 = tool_core::parse_wire_tool_call(
        "call-456".to_string(),
        "delete_all".to_string(),
        "{}".to_string(),
    )
    .unwrap();
    let denial = tool_core::build_denial_result(&tc2, "Policy denied: destructive operation");
    println!("  Denial for tool '{}': error = {:?}", tc2.name, denial.error);
    println!();

    // ─────────────────────────────────────────────────────────────
    // 3. Step Core: Usage Metadata Merging (pure function)
    // ─────────────────────────────────────────────────────────────
    println!("── Step Core: Merge Usage ──");
    let usage1 = UsageMetadata {
        prompt_token_count: Some(100),
        candidates_token_count: Some(50),
        total_token_count: Some(150),
        ..Default::default()
    };
    let usage2 = UsageMetadata {
        prompt_token_count: Some(200),
        candidates_token_count: Some(80),
        total_token_count: Some(280),
        ..Default::default()
    };
    let merged = step_core::merge_usage(Some(&usage1), &usage2);
    println!("  Turn 1: prompt={:?}, total={:?}", usage1.prompt_token_count, usage1.total_token_count);
    println!("  Turn 2: prompt={:?}, total={:?}", usage2.prompt_token_count, usage2.total_token_count);
    println!("  Merged: prompt={:?}, total={:?}", merged.prompt_token_count, merged.total_token_count);
    println!();

    // ─────────────────────────────────────────────────────────────
    // 4. Agent Core: Phase State Machine (pure enum)
    // ─────────────────────────────────────────────────────────────
    println!("── Agent Core: Phase State Machine ──");
    let phases = [
        agent_core::AgentPhase::Created,
        agent_core::AgentPhase::Starting,
        agent_core::AgentPhase::Running,
        agent_core::AgentPhase::Stopping,
        agent_core::AgentPhase::Stopped,
    ];
    for phase in &phases {
        let marker = if *phase == agent_core::AgentPhase::Running { "◀ active" } else { "" };
        println!("  {:?} {}", phase, marker);
    }
    println!();

    // ─────────────────────────────────────────────────────────────
    // 5. Pipeline: ROP Error Handling (pure types)
    // ─────────────────────────────────────────────────────────────
    println!("── Pipeline: ROP Error Handling ──");
    let err1 = pipeline::PipelineError::validation("Missing API key");
    println!("  Error 1: stage={:?}, msg=\"{}\"", err1.stage, err1.message);

    let err2 = pipeline::PipelineError::connection("WebSocket timeout");
    println!("  Error 2: stage={:?}, msg=\"{}\"", err2.stage, err2.message);

    let err3 = pipeline::PipelineError::tool_execution("Tool 'shell' returned exit code 1");
    println!("  Error 3: stage={:?}, msg=\"{}\"", err3.stage, err3.message);
    println!();

    // ─────────────────────────────────────────────────────────────
    // 6. Agent Events (append-only log)
    // ─────────────────────────────────────────────────────────────
    println!("── Agent Core: Event Sourcing ──");
    let events = vec![
        agent_core::AgentEvent::HookRunnerCreated { hook_count: 3 },
        agent_core::AgentEvent::ConnectionEstablished {
            conversation_id: "conv-abc-123".to_string(),
        },
        agent_core::AgentEvent::Started,
        agent_core::AgentEvent::Stopped,
    ];
    for (i, event) in events.iter().enumerate() {
        println!("  [{}] {:?}", i, event);
    }
    println!();

    println!("╔══════════════════════════════════════════╗");
    println!("║  All Functional Core demos completed ✅  ║");
    println!("╚══════════════════════════════════════════╝");
}
