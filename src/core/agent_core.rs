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

//! Pure agent state transition logic.
//!
//! All functions in this module are **pure**: they take immutable inputs
//! and return computed outputs without performing IO or mutating shared state.

use std::collections::HashSet;

use crate::types::{BuiltinTools, CapabilitiesConfig};

/// Result of safety policy validation.
#[derive(Debug, Clone)]
pub struct SafetyValidation {
    pub has_write_tools: bool,
    pub has_mcp_servers: bool,
    pub needs_policy: bool,
}

/// Errors from safety validation — pure, no IO.
#[derive(Debug, Clone)]
pub enum SafetyError {
    /// Write tools or MCP servers enabled without safety policy.
    MissingSafetyPolicy {
        has_write_tools: bool,
        has_mcp_servers: bool,
    },
}

impl std::fmt::Display for SafetyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSafetyPolicy {
                has_write_tools,
                has_mcp_servers,
            } => write!(
                f,
                "Write tools ({}) or MCP servers ({}) are enabled without a safety policy. \
                 Add policies=[policy::allow_all()] to approve all tool calls, \
                 or policies=[policy::deny_all(), policy::allow(\"tool_name\")] \
                 to selectively allow specific tools.",
                has_write_tools, has_mcp_servers
            ),
        }
    }
}

impl std::error::Error for SafetyError {}

/// Pure: Compute the set of active tools from capabilities config.
///
/// Applies the enabled/disabled tool filtering logic without any IO.
pub fn compute_active_tools(cfg: &CapabilitiesConfig) -> HashSet<BuiltinTools> {
    if let Some(ref enabled) = cfg.enabled_tools {
        enabled.iter().copied().collect()
    } else if let Some(ref disabled) = cfg.disabled_tools {
        let mut all: HashSet<_> = BuiltinTools::all_tools().into_iter().collect();
        for d in disabled {
            all.remove(d);
        }
        all
    } else {
        BuiltinTools::all_tools().into_iter().collect()
    }
}

/// Pure: Check if the active tool set contains any write tools.
pub fn has_write_tools(active: &HashSet<BuiltinTools>) -> bool {
    let read_only = BuiltinTools::read_only();
    active.iter().any(|t| !read_only.contains(t))
}

/// Pure: Validate that safety policy is correctly configured.
///
/// Returns `Ok(SafetyValidation)` if the configuration is valid,
/// or `Err(SafetyError)` if write tools/MCP are enabled without policies.
pub fn validate_safety(
    active_tools: &HashSet<BuiltinTools>,
    has_mcp: bool,
    policy_count: usize,
    has_decide_hooks: bool,
) -> Result<SafetyValidation, SafetyError> {
    let write_tools = has_write_tools(active_tools);
    let needs_policy = write_tools || has_mcp;

    if needs_policy && policy_count == 0 && !has_decide_hooks {
        return Err(SafetyError::MissingSafetyPolicy {
            has_write_tools: write_tools,
            has_mcp_servers: has_mcp,
        });
    }

    Ok(SafetyValidation {
        has_write_tools: write_tools,
        has_mcp_servers: has_mcp,
        needs_policy,
    })
}

/// Events emitted by agent state transitions.
///
/// These events form an append-only log that can be replayed
/// for debugging and state reconstruction.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// HookRunner created and initialized.
    HookRunnerCreated { hook_count: usize },
    /// Safety policy validated.
    SafetyValidated(SafetyValidation),
    /// Policies applied to hook runner.
    PoliciesApplied { policy_count: usize },
    /// MCP servers connected.
    McpConnected { server_count: usize },
    /// ToolRunner created with tools registered.
    ToolRunnerCreated { tool_count: usize },
    /// Connection established.
    ConnectionEstablished { conversation_id: String },
    /// Triggers started.
    TriggersStarted { count: usize },
    /// Agent fully started.
    Started,
    /// Agent stopped.
    Stopped,
}

/// Agent lifecycle phases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentPhase {
    /// Agent created but not yet started.
    Created,
    /// Agent is in the process of starting.
    Starting,
    /// Agent is running and ready for interaction.
    Running,
    /// Agent is in the process of stopping.
    Stopping,
    /// Agent has been stopped.
    Stopped,
}

impl Default for AgentPhase {
    fn default() -> Self {
        Self::Created
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_active_tools_default_returns_all() {
        let cfg = CapabilitiesConfig::default();
        let tools = compute_active_tools(&cfg);
        assert!(!tools.is_empty());
        // Should contain all tools
        let all = BuiltinTools::all_tools();
        assert_eq!(tools.len(), all.len());
    }

    #[test]
    fn test_compute_active_tools_enabled_filter() {
        let cfg = CapabilitiesConfig {
            enabled_tools: Some(vec![BuiltinTools::ViewFile]),
            disabled_tools: None,
            ..Default::default()
        };
        let tools = compute_active_tools(&cfg);
        assert_eq!(tools.len(), 1);
        assert!(tools.contains(&BuiltinTools::ViewFile));
    }

    #[test]
    fn test_compute_active_tools_disabled_filter() {
        let cfg = CapabilitiesConfig {
            disabled_tools: Some(vec![BuiltinTools::RunCommand]),
            ..Default::default()
        };
        let tools = compute_active_tools(&cfg);
        assert!(!tools.contains(&BuiltinTools::RunCommand));
    }

    #[test]
    fn test_has_write_tools_true() {
        let mut tools = HashSet::new();
        tools.insert(BuiltinTools::RunCommand); // Write tool
        assert!(has_write_tools(&tools));
    }

    #[test]
    fn test_has_write_tools_false() {
        let mut tools = HashSet::new();
        tools.insert(BuiltinTools::ViewFile); // Read-only tool
        assert!(!has_write_tools(&tools));
    }

    #[test]
    fn test_validate_safety_ok_with_policy() {
        let mut tools = HashSet::new();
        tools.insert(BuiltinTools::RunCommand);
        let result = validate_safety(&tools, false, 1, false);
        assert!(result.is_ok());
        let v = result.unwrap();
        assert!(v.has_write_tools);
        assert!(v.needs_policy);
    }

    #[test]
    fn test_validate_safety_ok_with_decide_hooks() {
        let mut tools = HashSet::new();
        tools.insert(BuiltinTools::RunCommand);
        let result = validate_safety(&tools, false, 0, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_safety_err_missing_policy() {
        let mut tools = HashSet::new();
        tools.insert(BuiltinTools::RunCommand);
        let result = validate_safety(&tools, false, 0, false);
        assert!(result.is_err());
        let e = result.unwrap_err();
        assert!(matches!(e, SafetyError::MissingSafetyPolicy { .. }));
    }

    #[test]
    fn test_validate_safety_err_mcp_without_policy() {
        let tools = HashSet::new(); // No write tools
        let result = validate_safety(&tools, true, 0, false); // But MCP servers
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_safety_ok_read_only() {
        let mut tools = HashSet::new();
        tools.insert(BuiltinTools::ViewFile);
        let result = validate_safety(&tools, false, 0, false);
        assert!(result.is_ok());
        assert!(!result.unwrap().needs_policy);
    }

    #[test]
    fn test_agent_phase_default() {
        assert_eq!(AgentPhase::default(), AgentPhase::Created);
    }

    #[test]
    fn test_safety_error_display() {
        let e = SafetyError::MissingSafetyPolicy {
            has_write_tools: true,
            has_mcp_servers: false,
        };
        let msg = format!("{e}");
        assert!(msg.contains("safety policy"));
        assert!(msg.contains("allow_all"));
    }
}
