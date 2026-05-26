<!-- disableFinding(LINK_RELATIVE_G3DOC) -->
<!-- disableFinding(LINE_OVER_80) -->

# Advanced Agent Configuration Guide

This guide provides instructions on how to perform advanced configuration for
Google Antigravity SDK agents.

## Model Selection

### Default Model

Google Antigravity SDK's default model is `gemini-3.5-flash`.

### Finding Valid Models

To find the most up-to-date list of valid Gemini model identifiers, refer to the
official documentation: -
[Google AI Studio Documentation](https://ai.google.dev/gemini-api/docs/models/gemini)

## CRITICAL RULE: Never Assume Valid Model Identifiers

> [!IMPORTANT] **Do not assume valid model identifiers.** Avoid guessing model
> names or assuming they follow a specific pattern. Always verify the valid
> identifiers from official documentation or user context before using them.

> [!IMPORTANT] **Avoid setting the model explicitly unless requested.** It is
> generally better to leave the model unset to use the default behavior, unless
> the user has explicitly requested a specific model.

## Advanced Configuration Examples

Here are small code snippets demonstrating advanced configurations using
`LocalAgentConfig`.

### Basic Configuration with Model Selection

```rust
use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = LocalAgentConfig {
        model: Some("gemini-3.5-flash".to_string()),
        ..Default::default()
    };
    let mut agent = Agent::new(config);
    agent.start().await?;
    // Use the agent
    agent.stop().await?;
    Ok(())
}
```

### Application Data Directory Override (Artifact & Scratch Storage)

By default, the agent stores generated artifacts (like `task.md`), scratch
files, and uploaded media under `~/.gemini/antigravity/brain/`. You can override
this location by specifying an absolute path in `app_data_dir`:

```rust
use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = LocalAgentConfig {
        app_data_dir: Some("/absolute/path/to/custom/storage".to_string()),
        ..Default::default()
    };
    let mut agent = Agent::new(config);
    agent.start().await?;
    // Generated files and artifacts will be written inside the custom directory
    agent.stop().await?;
    Ok(())
}
```

> [!IMPORTANT] **The path must be an absolute path.** Passing relative paths or
> unexpanded tildes (`~/`) will trigger a validation error.

### System Instructions and Personas

You can configure system instructions directly in the `LocalAgentConfig`:
```rust
let config = LocalAgentConfig {
    system_instructions: Some("You are an expert software architect.".to_string()),
    ..Default::default()
};
```
For a more detailed guide and complex persona examples,
see [persona_config.md](../../examples/getting_started/persona_config.md).

### Custom Tools

You can add custom tools to your agent:
```rust
let config = LocalAgentConfig {
    tools: vec![Box::new(MyCustomTool)],
    ..Default::default()
};
```
For a full
guide on creating and using custom tools, see
[custom_tool.md](../../examples/getting_started/custom_tools.md).

### MCP Integration

To configure Model Context Protocol (MCP) servers:
```rust
use std::collections::HashMap;

let mut mcp_servers = HashMap::new();
mcp_servers.insert("my_mcp_server".to_string(), "http://localhost:8080".to_string());

let config = LocalAgentConfig {
    mcp_servers,
    ..Default::default()
};
```
For
more details, see [mcp_integration.md](mcp_integration.md).
