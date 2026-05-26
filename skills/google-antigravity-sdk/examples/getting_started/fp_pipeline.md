# Example: fp_pipeline — Functional Core Pure Functions

This example demonstrates how to use the SDK's **Functional Core** (`src/core/`) directly.
All functions shown are **pure** — no IO, no async, fully deterministic and testable without mocks.

Run with:
```sh
cargo run --example fp_pipeline
```

## Source: `examples/getting_started/fp_pipeline.rs`

```rust
use antigravity_sdk::core::{agent_core, pipeline, step_core, tool_core};
use antigravity_sdk::types::UsageMetadata;

fn main() {
    // 1. Safety Validation (pure function)
    let active_tools = vec!["shell".to_string(), "read_file".to_string()];
    match agent_core::validate_safety(&active_tools, false, 1, true) {
        Ok(()) => println!("✅ Safety check passed"),
        Err(e) => println!("❌ Safety check failed: {}", e),
    }
    let has_writes = agent_core::has_write_tools(&active_tools);
    println!("Has write tools: {}", has_writes);

    // 2. Tool Call Parsing (pure function)
    let tc = tool_core::parse_wire_tool_call(
        "call-123".to_string(),
        "get_weather".to_string(),
        r#"{"city": "Tokyo"}"#.to_string(),
    );
    match tc {
        Ok(tc) => println!("✅ Parsed: {} (id={})", tc.name, tc.id),
        Err(e) => println!("❌ Parse failed: {}", e),
    }

    // 3. Usage Metadata Merging (pure function)
    let usage1 = UsageMetadata { prompt_token_count: Some(100), total_token_count: Some(150), ..Default::default() };
    let usage2 = UsageMetadata { prompt_token_count: Some(200), total_token_count: Some(280), ..Default::default() };
    let merged = step_core::merge_usage(Some(&usage1), &usage2);
    println!("Merged total tokens: {:?}", merged.total_token_count); // Some(430)

    // 4. AgentPhase State Machine (pure enum)
    let phase = agent_core::AgentPhase::Running;
    assert_eq!(phase, agent_core::AgentPhase::Running);

    // 5. Pipeline Error Types (ROP)
    let err = pipeline::PipelineError::validation("Missing API key");
    println!("Error stage: {:?}", err.stage); // Validation

    // 6. AgentEvent Log (Event Sourcing)
    let events = vec![
        agent_core::AgentEvent::HookRunnerCreated { hook_count: 3 },
        agent_core::AgentEvent::ConnectionEstablished { conversation_id: "conv-abc".to_string() },
        agent_core::AgentEvent::Started,
    ];
    for (i, event) in events.iter().enumerate() {
        println!("[{}] {:?}", i, event);
    }
}
```

## Key Concepts Demonstrated

### 1. `agent_core::validate_safety()` — Pure Safety Check
- Input: list of active tools, MCP flag, policy count, hook flag
- Output: `Ok(())` or `Err(PipelineError)`
- No side effects, no async — just input → validation → output

### 2. `tool_core::parse_wire_tool_call()` — Pure Parser
- Input: raw strings (id, name, arguments_json) from protobuf wire format
- Output: typed `ToolCall` struct or parse error
- Separates parsing (pure) from network IO (impure)

### 3. `step_core::merge_usage()` — Pure Accumulator
- Input: existing `Option<&UsageMetadata>` + new `&UsageMetadata`
- Output: merged `UsageMetadata` (sums all token counts)
- Used internally by `Conversation::push_step()` — extracted for testability

### 4. `agent_core::AgentPhase` — State Machine Type
- Pure enum: `Created → Starting → Running → Stopping → Stopped`
- Replaces `started: bool` with a proper 5-state machine
- Accessible via `agent.phase()` and `agent.events()`

### 5. `pipeline::PipelineError` — ROP Error Type
- Contains: `stage` (Validation/Connection/ToolExecution/HookDispatch), `message`, optional `source`
- Constructors: `PipelineError::validation()`, `::connection()`, `::tool_execution()`, `::hook_dispatch()`

### 6. `agent_core::AgentEvent` — Event Log Entry
- Variants: `HookRunnerCreated`, `ConnectionEstablished`, `Started`, `Stopped`
- Accessed via `agent.events()` — append-only, never modified

## Why Pure Functions Matter

| Aspect | Impure (old) | Pure (Functional Core) |
|:-------|:------------|:----------------------|
| Testing | Requires mock, async runtime | Just call the function |
| Debugging | Depends on runtime state | Same input = same output |
| Reuse | Tightly coupled to context | Composable anywhere |
| Documentation | Read the implementation | Signature is the contract |
