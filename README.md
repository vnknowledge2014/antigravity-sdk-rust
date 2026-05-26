# Google Antigravity SDK (Rust Edition)

> **Disclaimer:** This is an unofficial port of the Google Antigravity SDK from Python to Rust. It is not an official Google project.

The Google Antigravity SDK is a Rust SDK for building AI agents powered by
Antigravity and Gemini. It provides a secure, scalable, and stateful
infrastructure layer that abstracts the agentic loop, letting you focus on what
your agent *does* rather than how it runs.

## Installation

Add the SDK to your `Cargo.toml`:

```toml
[dependencies]
antigravity-sdk = { path = "path/to/antigravity-sdk" } # Or git repository URL
tokio = { version = "1", features = ["full"] }
```

> [!IMPORTANT]
> The Google Antigravity SDK relies on a Go localharness binary communicating via WebSocket and Protobuf. Ensure the harness is correctly set up.

## Quickstart

Get started by running one of the [`examples/`](examples/), such as the
`hello_world` example with:

```sh
export GEMINI_API_KEY="your_api_key_here"
cargo run --example hello_world
```
## Concepts

### Simple Agent

The `Agent` struct is the easiest way to get started. It manages the full
lifecycle — tool wiring, hook registration, and policy defaults.

```rust
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::Agent;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = LocalAgentConfig::default();
    let mut agent = Agent::new(config);
    
    agent.start().await?;
    
    let response = agent.chat("What files are in the current directory?").await?;
    println!("{}", response.text().await);
    
    agent.stop().await?;
    Ok(())
}
```

### Streaming Responses

To stream agent output in real-time (e.g., for fluid UI or console applications), simply consume the `ChatResponse` object which implements `futures::stream::Stream`:

```rust
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::Agent;
use futures::StreamExt;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut agent = Agent::new(LocalAgentConfig::default());
    agent.start().await?;

    let mut response = agent.chat("Write a short poem about space.").await?;
    
    while let Some(chunk) = response.next().await {
        print!("{}", chunk);
        io::stdout().flush()?;
    }
    println!();
    
    agent.stop().await?;
    Ok(())
}
```

By default, `Agent` runs in **read-only mode** for safety. Pass
`capabilities: CapabilitiesConfig::default()` to enable all tools (including writes).

### Interactive Loop

```rust
use antigravity_sdk::connections::local::{LocalAgentConfig, CapabilitiesConfig};
use antigravity_sdk::Agent;
use antigravity_sdk::utils::interactive::run_interactive_loop;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = LocalAgentConfig::default();
    config.capabilities = Some(CapabilitiesConfig::default());
    
    let mut agent = Agent::new(config);
    agent.start().await?;
    
    run_interactive_loop(&mut agent).await?;
    
    agent.stop().await?;
    Ok(())
}
```

### Advanced Usage with Conversation

For full control over the connection lifecycle, use `Conversation` directly:

```rust
use antigravity_sdk::connections::local::LocalConnectionStrategy;
use antigravity_sdk::conversation::Conversation;
use antigravity_sdk::tools::tool_runner::ToolRunner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tool_runner = ToolRunner::new();
    let mut strategy = LocalConnectionStrategy::new(
        Default::default(),
        tool_runner,
        None,
        None,
    );
    
    let mut conversation = Conversation::create(&mut strategy).await?;
    
    let response = conversation.chat("What files are here?").await?;
    println!("{}", response.text().await);
    
    println!("Total steps: {}", conversation.history().await.len());
    
    conversation.disconnect().await?;
    Ok(())
}
```

## Architecture

The SDK follows a state-of-the-art **Functional Programming** architecture:

- **Railway Oriented Programming (ROP)**: Pipeline-based error handling with `Pipeline<T>` and `PipelineError`
- **Functional Core – Imperative Shell**: Pure functions in `src/core/`, IO at boundaries
- **Actor Model**: Zero-lock concurrency via `StateActor` and `WriterActor` (replaces `Arc<Mutex<...>>`)
- **Event Sourcing**: `AgentPhase` state machine + `AgentEvent` append-only log

For detailed architecture documentation, see [FP Architecture](docs/05_fp_architecture.md).

## Features

### Custom Tools

Register Rust closures or functions as tools that the agent can call using `RegisteredTool`:

```rust
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::Agent;
use antigravity_sdk::tools::tool_runner::{RegisteredTool, ToolRunner};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut tool_runner = ToolRunner::new();
    let _ = tool_runner.register(RegisteredTool {
        name: "get_weather".to_string(),
        description: "Returns the current weather for a city.".to_string(),
        schema: Some(json!({
            "type": "object",
            "properties": {
                "city": { "type": "string" }
            },
            "required": ["city"]
        })),
        handler: Box::new(|args| {
            Box::pin(async move {
                let city = args.get("city").unwrap().as_str().unwrap();
                Ok(json!(format!("It's sunny in {}.", city)))
            })
        }),
    });

    let config = LocalAgentConfig::default();
    let mut agent = Agent::new(config);
    // Note: Tool Runner injection occurs automatically through hooks/config in advanced use
    agent.start().await?;
    agent.chat("What's the weather in Tokyo?").await?;
    agent.stop().await?;
    Ok(())
}
```

### MCP Integration

Connect to external [MCP](https://modelcontextprotocol.io/) servers and expose their tools to the agent natively using the `rmcp` crate integration.

### Hooks and Policies

Control agent behavior with a declarative policy system inside the `HookRunner` or via `AgentConfig`.

## Component Documentation

For more detailed documentation, see the deep-dive guides:

- [Getting Started](docs/01_getting_started.md) — Basics and Setup
- [Core Concepts](docs/02_core_concepts.md) — Agent, Conversation, Streaming
- [Advanced Usage](docs/03_advanced_usage.md) — Custom Tools, MCP, Policies, Triggers
- [Architecture](docs/04_architecture.md) — Under the hood of the Rust implementation
- [FP Architecture](docs/05_fp_architecture.md) — Railway Oriented Programming, Actor Model, Event Sourcing

## License

[Apache License 2.0](LICENSE)
