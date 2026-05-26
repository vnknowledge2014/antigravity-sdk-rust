<!-- disableFinding(LINK_RELATIVE_G3DOC) -->
<!-- disableFinding(LINE_OVER_80) -->
# Observability

This guide covers how to monitor costs and execution behavior of agents built
with the Google Antigravity SDK.

## Token Usage Tracking

You can track token usage across a session using the conversation object.
It returns a `UsageMetadata` struct containing cumulative counts for the session.

```rust
use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut agent = Agent::new(LocalAgentConfig::default());
    agent.start().await?;
    
    // Perform chat
    // ...
    
    // Retrieve usage (assuming conversation exposes it)
    // if let Some(usage) = agent.conversation().total_usage() {
    //     println!("Prompt tokens: {}", usage.prompt_token_count);
    //     println!("Candidates tokens: {}", usage.candidates_token_count);
    //     println!("Thoughts tokens: {}", usage.thoughts_token_count);
    //     println!("Total tokens: {}", usage.total_token_count);
    // }
    
    agent.stop().await?;
    Ok(())
}
```

The `UsageMetadata` object contains: * `prompt_token_count`: Number of tokens in
the prompt. * `cached_content_token_count`: Number of tokens from cached
content. * `candidates_token_count`: Number of tokens in the generated
candidates (excluding thinking). * `thoughts_token_count`: Number of tokens used
for thinking/reasoning. * `total_token_count`: Sum of prompt + candidates +
thinking tokens.

> [!IMPORTANT] **Thinking tokens** can significantly increase the total count
> unexpectedly, especially with models that support extended thinking. Always
> monitor `thoughts_token_count` if you are using thinking models.

> [!CAUTION] If the agent execution fails (e.g., due to an invalid API key or
> backend error), token usage counts may be reported as 0.

## Standard Logging

The SDK uses the `tracing` ecosystem. To see what the harness is doing, you can
enable `INFO` or `DEBUG` logging using `tracing-subscriber`.

```rust
// Enable INFO logging for the SDK
tracing_subscriber::fmt()
    .with_env_filter("antigravity_sdk=info")
    .init();
```

This will output information about session start/stop, connection establishment,
and tool execution. For more details on using logs for troubleshooting, see the
[Error Handling and Debugging](error_handling.md)
guide.

## Custom Tracing with Hooks

For advanced monitoring, you can use lifecycle hooks to build custom audit logs
or execution traces. For example, you can use `PostToolCallHook` to inspect the
results of every tool call.

```rust
use antigravity_sdk::hooks::{HookContext, PostToolCallHook};
use antigravity_sdk::connections::local::LocalAgentConfig;

pub struct AuditLogHook;

#[async_trait::async_trait]
impl PostToolCallHook for AuditLogHook {
    async fn run(&self, ctx: &HookContext, data: &serde_json::Value) {
        println!("[AUDIT] Tool execution completed. Result: {:?}", data);
    }
}

// Register the hook in your AgentConfig
let config = LocalAgentConfig {
    hooks: vec![Box::new(AuditLogHook)],
    ..Default::default()
};
```

You can also use `PreToolCallDecideHook` to log tool calls *before* they are
executed, or even block them based on custom logic. For a complete list of
available hooks and practical examples, see the
[Hooks Example](../examples/getting_started/hooks.md).
