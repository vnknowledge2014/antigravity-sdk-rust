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

//! Tool call policy system for the Antigravity SDK.
//!
//! Provides a declarative API for expressing tool call policies (APPROVE, DENY,
//! ASK_USER) that are enforced via the hooks system. Policies are evaluated
//! using a priority-based model where specificity and safety determine
//! precedence:
//!
//!   Specific Deny > Specific Ask > Specific Allow >
//!   Wildcard Deny > Wildcard Ask > Wildcard Allow
//!
//! Within each priority group, first match wins, enabling short-circuit
//! evaluation.

use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use tracing::info;

use crate::hooks::{DecideHook, HookContext};
use crate::types::{BuiltinTools, HookResult, ToolCall};

const WILDCARD: &str = "*";

/// Outcome a policy can produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Approve,
    Deny,
    AskUser,
}

/// A predicate on tool call arguments. Returns true if the policy applies.
pub type Predicate =
    Box<dyn Fn(&ToolCall) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> + Send + Sync>;

/// An ask_user handler. Returns true if the user approved execution.
pub type AskUserHandler =
    Box<dyn Fn(&ToolCall) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> + Send + Sync>;

/// A single tool call policy rule.
pub struct Policy {
    /// Tool name this policy targets, or "*" for all tools.
    pub tool: String,
    /// The outcome when this policy matches.
    pub decision: Decision,
    /// Optional predicate on the tool call. If None, matches any call.
    pub when: Option<Predicate>,
    /// Handler invoked when decision is AskUser.
    pub ask_user: Option<AskUserHandler>,
    /// Human-readable label.
    pub name: String,
}

// --- Builder helpers ---

/// Creates an APPROVE policy for `tool`.
pub fn allow(tool: impl Into<String>) -> Policy {
    Policy {
        tool: tool.into(),
        decision: Decision::Approve,
        when: None,
        ask_user: None,
        name: String::new(),
    }
}

/// Creates a DENY policy for `tool`.
pub fn deny(tool: impl Into<String>) -> Policy {
    Policy {
        tool: tool.into(),
        decision: Decision::Deny,
        when: None,
        ask_user: None,
        name: String::new(),
    }
}

/// Creates a policy that approves all tool calls without confirmation.
pub fn allow_all() -> Policy {
    let mut p = allow(WILDCARD);
    p.name = "allow_all".to_string();
    p
}

/// Creates a policy that denies all tool calls.
pub fn deny_all() -> Policy {
    let mut p = deny(WILDCARD);
    p.name = "deny_all".to_string();
    p
}

/// Safe default: allows all tools, denies run_command.
pub fn confirm_run_command() -> Vec<Policy> {
    vec![
        {
            let mut p = deny(BuiltinTools::RunCommand.as_str());
            p.name = "confirm_run_command".to_string();
            p
        },
        {
            let mut p = allow(WILDCARD);
            p.name = "confirm_run_command".to_string();
            p
        },
    ]
}

// --- Priority buckets ---

const LEVEL_SPECIFIC_DENY: usize = 0;
const LEVEL_SPECIFIC_ASK: usize = 1;
const LEVEL_SPECIFIC_ALLOW: usize = 2;
const LEVEL_WILDCARD_DENY: usize = 3;
const LEVEL_WILDCARD_ASK: usize = 4;
const LEVEL_WILDCARD_ALLOW: usize = 5;
const NUM_LEVELS: usize = 6;

fn bucket_index(p: &Policy) -> usize {
    let is_wildcard = p.tool == WILDCARD;
    match (is_wildcard, p.decision) {
        (false, Decision::Deny) => LEVEL_SPECIFIC_DENY,
        (false, Decision::AskUser) => LEVEL_SPECIFIC_ASK,
        (false, Decision::Approve) => LEVEL_SPECIFIC_ALLOW,
        (true, Decision::Deny) => LEVEL_WILDCARD_DENY,
        (true, Decision::AskUser) => LEVEL_WILDCARD_ASK,
        (true, Decision::Approve) => LEVEL_WILDCARD_ALLOW,
    }
}

fn matches_tool(policy: &Policy, tool_name: &str) -> bool {
    policy.tool == WILDCARD || policy.tool == tool_name
}

// --- Hook implementation ---

/// PreToolCallDecideHook that enforces a set of policies.
pub struct PolicyDecideHook {
    buckets: Vec<Vec<Policy>>,
}

#[async_trait]
impl DecideHook<ToolCall> for PolicyDecideHook {
    async fn run(&self, _context: &mut HookContext, data: &ToolCall) -> HookResult {
        let tool_name = data.name.to_string();

        for bucket in &self.buckets {
            for p in bucket {
                if !matches_tool(p, &tool_name) {
                    continue;
                }

                // Evaluate predicate
                if let Some(ref pred) = p.when
                    && !(pred)(data).await
                {
                    continue;
                }

                let label = if p.name.is_empty() { &p.tool } else { &p.name };

                match p.decision {
                    Decision::Deny => {
                        info!("Policy '{}' denied tool '{}'.", label, tool_name);
                        return HookResult::denied(format!("Denied by policy '{label}'."));
                    }
                    Decision::Approve => {
                        info!("Policy '{}' approved tool '{}'.", label, tool_name);
                        return HookResult::allowed();
                    }
                    Decision::AskUser => {
                        info!(
                            "Policy '{}' requesting user approval for tool '{}'.",
                            label, tool_name
                        );
                        if let Some(ref handler) = p.ask_user {
                            if handler(data).await {
                                return HookResult::allowed();
                            }
                            return HookResult::denied(format!(
                                "User denied tool '{tool_name}' (policy '{label}')."
                            ));
                        }
                        // No handler — deny
                        return HookResult::denied(format!(
                            "ASK_USER policy '{label}' has no handler."
                        ));
                    }
                }
            }
        }

        // No policy matched — default open.
        HookResult::allowed()
    }
}

/// Creates a PreToolCallDecideHook that enforces the given policies.
pub fn enforce(policies: Vec<Policy>) -> PolicyDecideHook {
    let mut buckets: Vec<Vec<Policy>> = (0..NUM_LEVELS).map(|_| Vec::new()).collect();
    for p in policies {
        let idx = bucket_index(&p);
        buckets[idx].push(p);
    }
    PolicyDecideHook { buckets }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolName;
    use std::collections::HashMap;

    fn make_tool_call(name: &str) -> ToolCall {
        ToolCall {
            name: ToolName::Custom(name.to_string()),
            args: HashMap::new(),
            id: Some("tc-1".to_string()),
            canonical_path: None,
        }
    }

    fn make_builtin_tool_call(builtin: BuiltinTools) -> ToolCall {
        ToolCall {
            name: ToolName::Builtin(builtin),
            args: HashMap::new(),
            id: Some("tc-1".to_string()),
            canonical_path: None,
        }
    }

    // --- Builder helpers ---

    #[test]
    fn test_allow_creates_approve_policy() {
        let p = allow("my_tool");
        assert_eq!(p.tool, "my_tool");
        assert_eq!(p.decision, Decision::Approve);
        assert!(p.when.is_none());
        assert!(p.ask_user.is_none());
    }

    #[test]
    fn test_deny_creates_deny_policy() {
        let p = deny("my_tool");
        assert_eq!(p.tool, "my_tool");
        assert_eq!(p.decision, Decision::Deny);
    }

    #[test]
    fn test_allow_all_is_wildcard() {
        let p = allow_all();
        assert_eq!(p.tool, "*");
        assert_eq!(p.decision, Decision::Approve);
        assert_eq!(p.name, "allow_all");
    }

    #[test]
    fn test_deny_all_is_wildcard() {
        let p = deny_all();
        assert_eq!(p.tool, "*");
        assert_eq!(p.decision, Decision::Deny);
        assert_eq!(p.name, "deny_all");
    }

    #[test]
    fn test_confirm_run_command_returns_two_policies() {
        let policies = confirm_run_command();
        assert_eq!(policies.len(), 2);
        assert_eq!(policies[0].tool, "run_command");
        assert_eq!(policies[0].decision, Decision::Deny);
        assert_eq!(policies[1].tool, "*");
        assert_eq!(policies[1].decision, Decision::Approve);
    }

    // --- Bucket indexing ---

    #[test]
    fn test_bucket_index_specific_deny() {
        let p = deny("foo");
        assert_eq!(bucket_index(&p), LEVEL_SPECIFIC_DENY);
    }

    #[test]
    fn test_bucket_index_wildcard_approve() {
        let p = allow("*");
        assert_eq!(bucket_index(&p), LEVEL_WILDCARD_ALLOW);
    }

    #[test]
    fn test_bucket_index_wildcard_deny() {
        let p = deny("*");
        assert_eq!(bucket_index(&p), LEVEL_WILDCARD_DENY);
    }

    // --- matches_tool ---

    #[test]
    fn test_matches_tool_exact() {
        let p = allow("my_tool");
        assert!(matches_tool(&p, "my_tool"));
        assert!(!matches_tool(&p, "other_tool"));
    }

    #[test]
    fn test_matches_tool_wildcard() {
        let p = allow("*");
        assert!(matches_tool(&p, "any_tool"));
        assert!(matches_tool(&p, "another_tool"));
    }

    // --- Enforce + dispatch ---

    #[tokio::test]
    async fn test_enforce_allow_all() {
        let hook = enforce(vec![allow_all()]);
        let mut ctx = HookContext::new();
        let result = hook.run(&mut ctx, &make_tool_call("any_tool")).await;
        assert!(result.allow);
    }

    #[tokio::test]
    async fn test_enforce_deny_all() {
        let hook = enforce(vec![deny_all()]);
        let mut ctx = HookContext::new();
        let result = hook.run(&mut ctx, &make_tool_call("any_tool")).await;
        assert!(!result.allow);
        assert!(result.message.contains("deny_all"));
    }

    #[tokio::test]
    async fn test_enforce_no_policies_allows() {
        let hook = enforce(vec![]);
        let mut ctx = HookContext::new();
        let result = hook.run(&mut ctx, &make_tool_call("my_tool")).await;
        assert!(result.allow); // Default open
    }

    #[tokio::test]
    async fn test_specific_deny_overrides_wildcard_allow() {
        let hook = enforce(vec![deny("dangerous_tool"), allow_all()]);
        let mut ctx = HookContext::new();

        // Specific deny should block "dangerous_tool"
        let result = hook.run(&mut ctx, &make_tool_call("dangerous_tool")).await;
        assert!(!result.allow);

        // Other tools should be allowed
        let result = hook.run(&mut ctx, &make_tool_call("safe_tool")).await;
        assert!(result.allow);
    }

    #[tokio::test]
    async fn test_confirm_run_command_policy() {
        let hook = enforce(confirm_run_command());
        let mut ctx = HookContext::new();

        // run_command should be denied
        let result = hook
            .run(&mut ctx, &make_builtin_tool_call(BuiltinTools::RunCommand))
            .await;
        assert!(!result.allow);

        // Other tools should be allowed
        let result = hook
            .run(&mut ctx, &make_builtin_tool_call(BuiltinTools::ViewFile))
            .await;
        assert!(result.allow);
    }

    #[tokio::test]
    async fn test_enforce_with_predicate() {
        let hook = enforce(vec![
            {
                let mut p = deny("risky");
                p.when = Some(Box::new(|tc| {
                    let has_force = tc.args.contains_key("force");
                    Box::pin(async move { has_force })
                }));
                p
            },
            allow_all(),
        ]);
        let mut ctx = HookContext::new();

        // Without "force" arg — predicate fails, falls through to allow_all
        let result = hook.run(&mut ctx, &make_tool_call("risky")).await;
        assert!(result.allow);

        // With "force" arg — predicate matches, denied
        let mut tc_with_force = make_tool_call("risky");
        tc_with_force
            .args
            .insert("force".to_string(), serde_json::json!(true));
        let result = hook.run(&mut ctx, &tc_with_force).await;
        assert!(!result.allow);
    }

    #[tokio::test]
    async fn test_ask_user_with_handler_approved() {
        let hook = enforce(vec![{
            
            Policy {
                tool: "interactive".to_string(),
                decision: Decision::AskUser,
                when: None,
                ask_user: Some(Box::new(|_tc| Box::pin(async { true }))),
                name: "ask_policy".to_string(),
            }
        }]);
        let mut ctx = HookContext::new();
        let result = hook.run(&mut ctx, &make_tool_call("interactive")).await;
        assert!(result.allow);
    }

    #[tokio::test]
    async fn test_ask_user_with_handler_denied() {
        let hook = enforce(vec![Policy {
            tool: "interactive".to_string(),
            decision: Decision::AskUser,
            when: None,
            ask_user: Some(Box::new(|_tc| Box::pin(async { false }))),
            name: "ask_policy".to_string(),
        }]);
        let mut ctx = HookContext::new();
        let result = hook.run(&mut ctx, &make_tool_call("interactive")).await;
        assert!(!result.allow);
        assert!(result.message.contains("User denied"));
    }

    #[tokio::test]
    async fn test_ask_user_without_handler_denies() {
        let hook = enforce(vec![Policy {
            tool: "interactive".to_string(),
            decision: Decision::AskUser,
            when: None,
            ask_user: None,
            name: "no_handler".to_string(),
        }]);
        let mut ctx = HookContext::new();
        let result = hook.run(&mut ctx, &make_tool_call("interactive")).await;
        assert!(!result.allow);
        assert!(result.message.contains("no handler"));
    }

    #[tokio::test]
    async fn test_priority_order_specific_deny_beats_specific_allow() {
        // Even though allow comes first in input order, deny has higher priority
        let hook = enforce(vec![allow("my_tool"), deny("my_tool")]);
        let mut ctx = HookContext::new();
        let result = hook.run(&mut ctx, &make_tool_call("my_tool")).await;
        assert!(!result.allow); // Deny wins due to bucket priority
    }

    #[tokio::test]
    async fn test_unmatched_tool_defaults_to_allow() {
        let hook = enforce(vec![deny("specific_tool")]);
        let mut ctx = HookContext::new();
        // "other_tool" has no matching policy — default open
        let result = hook.run(&mut ctx, &make_tool_call("other_tool")).await;
        assert!(result.allow);
    }
}
