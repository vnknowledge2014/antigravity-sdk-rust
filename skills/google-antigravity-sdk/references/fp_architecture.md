# Functional Programming Architecture

The Antigravity SDK (Rust Edition) follows a state-of-the-art Functional Programming architecture built on four pillars:

## 1. Railway Oriented Programming (ROP)

**Module**: `src/core/pipeline.rs`

Pipeline-based error handling where each step returns `Ok(T)` (success track) or `Err(PipelineError)` (error track). Key types:

- `PipelineError`: Contains `stage` (Validation, Connection, ToolExecution, HookDispatch), `message`, and optional `source` error
- `Pipeline<T>`: Wrapper for `Result<T, PipelineError>` enabling method chaining
- `ToolPipelineState`: Intermediate state in tool processing pipelines

**Impact on code**:
- `Agent::start()` refactored from 117-line monolith → 8 named pipeline steps
- `handle_tool_call()` refactored from 77 lines/5-level nesting → 3-step pipeline

## 2. Functional Core – Imperative Shell (FC-IS)

**Module**: `src/core/` (5 sub-modules)

All pure computation extracted into `src/core/`. These functions have **zero IO, zero side effects** and are fully deterministic:

| Module | Key Functions |
|:-------|:-------------|
| `agent_core.rs` | `compute_active_tools()`, `validate_safety()`, `has_write_tools()`, `AgentPhase`, `AgentEvent` |
| `tool_core.rs` | `parse_wire_tool_call()`, `build_denial_result()`, `resolve_tool_result()`, `effective_deny_message()` |
| `policy_core.rs` | `bucket_index()`, `matches_tool()`, `build_buckets()` |
| `step_core.rs` | `merge_usage()`, `add_option()` |
| `pipeline.rs` | `PipelineError`, `Pipeline<T>`, `ToolPipelineState` |

The "Imperative Shell" (`agent.rs`, `local.rs`, `conversation.rs`) orchestrates IO and calls into the pure core.

## 3. Actor Model

**Module**: `src/actors/`

Replaces the 8 `Arc<Mutex<...>>` fields in `LocalConnection` with message-passing actors:

| Actor | Purpose | Replaces |
|:------|:--------|:---------|
| `StateActor` | Central state owner — processes `StateMsg` commands/queries sequentially | 8 Mutex-wrapped fields |
| `WriterActor` | Dedicated WebSocket write actor | `Arc<Mutex<WsSink>>` |

Communication via `tokio::sync::mpsc` channels + `oneshot` for query responses.

### StateMsg Variants
- **Commands** (fire-and-forget): `StepReceived`, `SetCascadeId`, `AddSubagent`, `AppendStderr`, `SetTurnContext`
- **Queries** (with reply): `GetCascadeId`, `HasTurnContext`, `GetSnapshot`, `GetActiveSubagentCount`

### Zero-Lock Guarantee
Each actor runs in exactly one `tokio::spawn` task, owning all state fields outright. No `Mutex` or `RwLock` needed.

## 4. Event Sourcing

**Module**: `src/core/agent_core.rs` (types) + `src/agent.rs` (integration)

### AgentPhase State Machine
```
Created → Starting → Running → Stopping → Stopped
```

### AgentEvent Log
Append-only event log for debugging and replay:
- `HookRunnerCreated { hook_count }` — hooks registered
- `ConnectionEstablished { conversation_id }` — connection live
- `Started` — agent fully operational
- `Stopped` — agent shut down

Access via `agent.phase()` and `agent.events()`.
