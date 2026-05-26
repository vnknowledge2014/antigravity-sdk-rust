# Safety Policies in Google Antigravity SDK

Reference guide for configuring access control and safety policies in the Google
Antigravity SDK.

## Overview

The Google Antigravity SDK provides a declarative policy system to control which
tools an agent can execute. Policies are evaluated using a priority-based model
to ensure safety and prevent unauthorized actions.

## Default Behavior

By default, `LocalAgentConfig` uses `Policy::confirm_run_command()` which:

-   **Denies** `run_command` (shell execution is blocked)
-   **Allows** all other tools (view, edit, create files, etc.)

This means new agents are **conservative by default** — they cannot execute shell
commands unless you explicitly opt in.

If `workspaces` is set on the config, `Policy::workspace_only()` is also
automatically prepended, restricting file tools (`view_file`, `create_file`,
`edit_file`) to the configured workspace directories.

### Restoring Permissive Behavior

To allow all tools (including `run_command`), pass `Policy::allow_all()`:

```rust
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::hooks::policy::Policy;

let config = LocalAgentConfig {
    system_instructions: Some("You are a helpful assistant.".to_string()),
    policies: vec![Policy::allow_all()],
    ..Default::default()
};
```

## Policy Resolution Order

Policies are evaluated in the following order of precedence (highest to lowest):

1. **Specific Deny**: `Policy::deny("tool_name", ...)`
2. **Specific Ask**: `Policy::ask_user("tool_name", ...)`
3. **Specific Allow**: `Policy::allow("tool_name", ...)`
4. **Wildcard Deny**: `Policy::deny("*", ...)`
5. **Wildcard Ask**: `Policy::ask_user("*", ...)`
6. **Wildcard Allow**: `Policy::allow("*", ...)`

Within each priority group, the **first match wins** (short-circuit evaluation).

## Configuration

Use the `antigravity_sdk::hooks::policy` module to define policies.

### Allow

Approves tool calls without confirmation.

```rust
use antigravity_sdk::hooks::policy::Policy;

// Allow all calls to view_file
Policy::allow("view_file");
```

### Deny

Blocks tool calls immediately.

```rust
use antigravity_sdk::hooks::policy::Policy;

// Deny all calls to run_command
Policy::deny("run_command");
```

### Wildcards

-   `Policy::allow_all()`: Approves all tool calls. Equivalent to `allow("*")`.
-   `Policy::deny_all()`: Denies all tool calls. Equivalent to `deny("*")`.

### Convenience Presets

-   `Policy::confirm_run_command()`: Denies `run_command`, allows everything else.
    This is the **default** policy. 
-   `Policy::workspace_only(workspaces)`: Restricts `view_file`, `create_file`,
    and `edit_file` to paths within the given workspace directories.
    Automatically applied when `LocalAgentConfig.workspaces` is set.

## Minimal Safe Templates

### Deny by Default (Recommended for Production)

Start by denying everything and selectively allow safe tools.

```rust
use antigravity_sdk::connections::local::{LocalAgentConfig, CapabilitiesConfig};
use antigravity_sdk::hooks::policy::Policy;

let policies = vec![
    Policy::deny_all(),
    Policy::allow("view_file"),
    Policy::allow("code_search"),
];

let config = LocalAgentConfig {
    system_instructions: Some("You are a helpful assistant.".to_string()),
    capabilities: CapabilitiesConfig::default(),  // Enables write tools
    policies,
    ..Default::default()
};
```

### Safe Default (No Configuration Needed)

The default `confirm_run_command()` policy is suitable for most use cases. Simply
create a config without specifying policies:

```rust
use antigravity_sdk::connections::local::LocalAgentConfig;

// run_command is denied, all other tools allowed
let config = LocalAgentConfig {
    system_instructions: Some("You are a helpful assistant.".to_string()),
    ..Default::default()
};
```

### Allow All (Development Only)

Use only for local development where safety is not a concern.

```rust
use antigravity_sdk::connections::local::{LocalAgentConfig, CapabilitiesConfig};
use antigravity_sdk::hooks::policy::Policy;

let config = LocalAgentConfig {
    system_instructions: Some("You are a helpful assistant.".to_string()),
    capabilities: CapabilitiesConfig::default(),
    policies: vec![Policy::allow_all()],
    ..Default::default()
};
```
