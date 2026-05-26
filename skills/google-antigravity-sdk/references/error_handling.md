<!-- disableFinding(LINK_RELATIVE_G3DOC) -->
<!-- disableFinding(LINE_OVER_80) -->

# Error Handling and Debugging in Google Antigravity SDK

This guide is intended for agents assisting users in troubleshooting their
Google Antigravity agents and making them more robust.

## Part 1: Debugging & Troubleshooting

When an agent fails or behaves unexpectedly, follow these steps to help the user debug.

### Finding Why It Failed

1.  **Inspect Agent Thoughts**: If the interface or logs expose the agent's internal monologue or "thoughts", examine them to understand what it was trying to do before the failure. See [hello_world.md](../examples/getting_started/hello_world.md) for how to stream thoughts.
2.  **Stream Logs**: Check the streaming logs (e.g., WebSocket connection logs, agent execution logs). These often contain the raw error messages and tracebacks. To see these logs in your console, you need to configure Python's root logger at the beginning of your script. Since the SDK uses standard Python logging, these settings apply globally to all SDK logs:

```rust
// Configure the tracing subscriber to show INFO level messages and above
tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();
```

For custom observability, users can use lifecycle hooks (like `PostToolCallHook` or `OnInteractionHook`) to create their own structured audit logs or execution traces. See [hooks.md](../examples/getting_started/hooks.md) for how to implement these.

---

## Part 2: Error Management

To make agents robust, they should handle errors gracefully and potentially recover from them.

### Catching Exceptions

The SDK provides specific Error enums that you can match in your application code:

*   **`AntigravityError::ValidationError`**: Returned when input validation fails (e.g., invalid parameters passed to a tool or configuration).
*   **`AntigravityError::ConnectionError`**: Returned when connection issues occur (e.g., WebSocket drops, timeout).

Example:

```rust
use antigravity_sdk::error::AntigravityError;

match agent.start().await {
    Ok(_) => {},
    Err(AntigravityError::ValidationError(e)) => println!("Validation failed: {}", e),
    Err(AntigravityError::ConnectionError(e)) => println!("Connection failed: {}", e),
    Err(e) => println!("Other error: {}", e),
}
```

### Using Hooks for Error Recovery

You can use the `OnToolErrorHook` to intercept failures in tool execution. This is powerful because it allows the agent to see a fallback value instead of a raw error, potentially allowing it to self-correct and continue the conversation.

Here is a minimal example of implementing a fallback hook:

```rust
use antigravity_sdk::hooks::{HookContext, OnToolErrorHook};

pub struct FallbackHook;

#[async_trait::async_trait]
impl OnToolErrorHook for FallbackHook {
    async fn run(&self, ctx: &HookContext, err: &dyn std::error::Error) -> Option<String> {
        Some("[Could not complete operation. Please try with alternative parameters.]".to_string())
    }
}
```

To use this hook, add it to the `hooks` list in your `LocalAgentConfig`. See [hooks.md](../examples/getting_started/hooks.md) doc so the agent can discover more info if it wants.

```rust
use antigravity_sdk::connections::local::LocalAgentConfig;

let config = LocalAgentConfig {
    hooks: vec![Box::new(FallbackHook)],
    ..Default::default()
};
```
