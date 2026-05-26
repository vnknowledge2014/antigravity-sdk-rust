# Actor Model Concurrency

The Antigravity SDK uses an **Actor Model** pattern for safe, lock-free concurrency. This replaces the traditional `Arc<Mutex<T>>` ("Mutex Soup") approach.

## The Problem: Mutex Soup

The original `LocalConnection` had **8 fields** wrapped in `Arc<Mutex<...>>`:

```rust
// BEFORE: Mutex Soup — every access requires lock acquisition
pub struct LocalConnection {
    ws_writer: Arc<Mutex<UnboundedSender<Vec<u8>>>>,
    cascade_id: Arc<RwLock<Option<String>>>,
    current_turn_context: Arc<Mutex<Option<TurnContext>>>,
    step_trackers: Arc<Mutex<HashMap<(String, u32), StepTracker>>>,
    active_subagent_ids: Arc<Mutex<HashSet<String>>>,
    subagent_responses: Arc<Mutex<HashMap<String, String>>>,
    stderr_lines: Arc<Mutex<VecDeque<String>>>,
    pending_builtin_tool_calls: Arc<Mutex<HashMap<(String, u32), PendingCallValue>>>,
}
```

Problems with this pattern:
- **Lock contention**: Multiple tasks compete for the same locks
- **Deadlock risk**: Complex lock ordering required
- **Poor composability**: Can't atomically update multiple fields
- **Scattered state**: State spread across 8 separate lock domains

## The Solution: Actor Model

### StateActor

A single `tokio::spawn` task owns ALL mutable state:

```rust
// AFTER: Actor owns everything — zero locks needed
pub struct StateActor {
    turn_context: Option<TurnContext>,        // Was Arc<Mutex<...>>
    active_subagent_ids: HashSet<String>,      // Was Arc<Mutex<...>>
    subagent_responses: HashMap<String, String>, // Was Arc<Mutex<...>>
    stderr_lines: VecDeque<String>,            // Was Arc<Mutex<...>>
    cascade_id: Option<String>,               // Was Arc<RwLock<...>>
    step_count: usize,                        // New
    msg_rx: mpsc::UnboundedReceiver<StateMsg>, // Message inbox
    step_tx: mpsc::UnboundedSender<Step>,      // Output channel
}
```

### Communication Protocol

Other components send messages via `mpsc::UnboundedSender<StateMsg>`:

```rust
// Fire-and-forget commands
state_tx.send(StateMsg::AddSubagent("sub-1".into())).unwrap();
state_tx.send(StateMsg::AppendStderr("error line".into())).unwrap();

// Query with reply (uses oneshot channel)
let cascade_id = query(&state_tx, StateMsg::GetCascadeId).await;
let snapshot = query(&state_tx, StateMsg::GetSnapshot).await;
```

### WriterActor

Dedicated actor for WebSocket writes:

```rust
pub struct WriterActor {
    rx: mpsc::UnboundedReceiver<WriteMsg>,
    sink: mpsc::UnboundedSender<Vec<u8>>,
}
```

Processes `WriteMsg::Send(bytes)` and `WriteMsg::Shutdown` sequentially.

## Benefits

| Aspect | Mutex Soup | Actor Model |
|:-------|:-----------|:------------|
| Locks needed | 8 Mutex/RwLock | 0 |
| Deadlock risk | High | None |
| State atomicity | Per-field only | Full (sequential processing) |
| Testability | Requires Arc setup | Plain struct, no locks |
| Composability | Lock ordering | Message ordering |

## Key Files

- `src/actors/mod.rs` — Module definition
- `src/actors/state_actor.rs` — StateActor + StateMsg + query() helper
- `src/actors/writer_actor.rs` — WriterActor + WriteMsg
