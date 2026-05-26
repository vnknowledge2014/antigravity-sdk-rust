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
use antigravity_sdk::types::{BuiltinTools, UsageMetadata};
use std::collections::HashSet;

fn main() {
    println!("╔══════════════════════════════════════════╗");
    println!("║   Functional Core Demo — Pure Functions  ║");
    println!("╚══════════════════════════════════════════╝");
    println!();

    // ─────────────────────────────────────────────────────────────
    // 1. Agent Core: has_write_tools + validate_safety (pure)
    // ─────────────────────────────────────────────────────────────
    println!("── Agent Core: Safety Validation ──");
    // has_write_tools: takes &HashSet<BuiltinTools> — read-only tools
    let read_only: HashSet<BuiltinTools> = [BuiltinTools::ListDir, BuiltinTools::ViewFile]
        .into_iter()
        .collect();
    println!("  Read-only set has_write_tools: {}", agent_core::has_write_tools(&read_only));

    // Write-capable tools
    let with_writes: HashSet<BuiltinTools> = [BuiltinTools::RunCommand, BuiltinTools::EditFile]
        .into_iter()
        .collect();
    println!("  RunCommand+EditFile has_write_tools: {}", agent_core::has_write_tools(&with_writes));

    // validate_safety with policy — Ok
    match agent_core::validate_safety(&with_writes, false, 1, false) {
        Ok(v) => println!("  ✅ Safety ok (has_write_tools={})", v.has_write_tools),
        Err(e) => println!("  ❌ Safety fail: {}", e),
    }
    // validate_safety without policy for write tools — Err expected
    match agent_core::validate_safety(&with_writes, false, 0, false) {
        Ok(_) => println!("  (unexpected ok)"),
        Err(e) => println!("  Expected deny (no policy): {}", e),
    }
    println!();

    // ─────────────────────────────────────────────────────────────
    // 2. Tool Core: Parse Wire Tool Call (pure function)
    // ─────────────────────────────────────────────────────────────
    println!("── Tool Core: Parse Wire Tool Call ──");
    // API: Option<String> for each arg
    let tc = tool_core::parse_wire_tool_call(
        Some("call-123".to_string()),
        Some("get_weather".to_string()),
        Some(r#"{"city": "Tokyo"}"#.to_string()),
    );
    match &tc {
        Ok(tc) => println!("  ✅ Parsed: {:?} (id={:?})", tc.name, tc.id),
        Err(e) => println!("  ❌ Parse failed: {}", e),
    }

    // Build a denial result (pure function)
    if let Ok(ref tc2) = tool_core::parse_wire_tool_call(
        Some("call-456".to_string()),
        Some("delete_all".to_string()),
        None,
    ) {
        let denial = tool_core::build_denial_result(tc2, "Policy denied: destructive operation");
        println!("  Denial error: {:?}", denial.error);
    }
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
    println!("  Turn 1 total: {:?}", usage1.total_token_count);
    println!("  Turn 2 total: {:?}", usage2.total_token_count);
    println!("  Merged total: {:?}", merged.total_token_count); // Some(430)
    println!();

    // ─────────────────────────────────────────────────────────────
    // 4. Agent Core: Phase State Machine (pure enum, PartialEq)
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
        let marker = if *phase == agent_core::AgentPhase::Running { " ◀ active" } else { "" };
        println!("  {:?}{}", phase, marker);
    }
    println!();

    // ─────────────────────────────────────────────────────────────
    // 5. Pipeline: ROP Error Variants (pure types)
    // ─────────────────────────────────────────────────────────────
    println!("── Pipeline: ROP Error Variants ──");
    let e1 = pipeline::PipelineError::ValidationError("Missing API key".into());
    println!("  ValidationError: {}", e1);

    let e2 = pipeline::PipelineError::Denied {
        message: "policy denied shell tool".into(),
        tool_call: None,
    };
    println!("  Denied: {}", e2);

    let e3 = pipeline::PipelineError::ToolError {
        name: "shell".into(),
        error: "exit code 1".into(),
        recovery: None,
    };
    println!("  ToolError: {}", e3);

    // Pipeline chaining (ROP pattern)
    let result: pipeline::Pipeline<i32> = Ok(10)
        .and_then(|x| Ok(x * 2))
        .and_then(|x| Ok(x + 5));
    println!("  Pipeline(10 * 2 + 5): {:?}", result);
    println!();

    // ─────────────────────────────────────────────────────────────
    // 6. Agent Events (event sourcing — append-only log types)
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
