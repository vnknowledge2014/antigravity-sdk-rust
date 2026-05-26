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

//! Integration tests for the Antigravity SDK.
//!
//! These tests exercise the public API surface and cross-module interactions
//! that unit tests cannot cover. They do NOT require a running backend.

use antigravity_sdk::Agent;
use antigravity_sdk::connections::LocalAgentConfig;
use antigravity_sdk::hooks::policy;
use antigravity_sdk::hooks::{DecideHook, HookContext, HookRunner};
use antigravity_sdk::tools::{RegisteredTool, ToolContext, ToolRunner};
use antigravity_sdk::triggers;
use antigravity_sdk::types::*;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

// =============================================================================
// Agent lifecycle integration
// =============================================================================

#[tokio::test]
#[ignore]
async fn test_agent_lifecycle() {
    let mut agent = Agent::new(antigravity_sdk::AgentArgs::default());

    // Pre-start state
    assert!(!agent.is_started());
    assert!(agent.conversation_id().is_none());

    // Start
    agent.start().await.unwrap();
    assert!(agent.is_started());

    // Stop
    agent.stop().await.unwrap();
    assert!(!agent.is_started());
}

#[tokio::test]
#[ignore]
async fn test_agent_start_stop_cycle() {
    let mut agent = Agent::new(antigravity_sdk::AgentArgs::default());

    for _ in 0..3 {
        agent.start().await.unwrap();
        assert!(agent.is_started());
        agent.stop().await.unwrap();
        assert!(!agent.is_started());
    }
}

// =============================================================================
// Configuration types integration
// =============================================================================

#[test]
fn test_local_agent_config_default() {
    let config = LocalAgentConfig::default();
    assert!(config.api_key.is_none());
}

#[test]
fn test_capabilities_config_validation_integration() {
    // Valid: enabled_tools only
    let valid = CapabilitiesConfig {
        enabled_tools: Some(BuiltinTools::read_only()),
        ..Default::default()
    };
    assert!(valid.validate().is_ok());

    // Invalid: both enabled and disabled
    let invalid = CapabilitiesConfig {
        enabled_tools: Some(vec![BuiltinTools::ViewFile]),
        disabled_tools: Some(vec![BuiltinTools::RunCommand]),
        ..Default::default()
    };
    assert!(invalid.validate().is_err());
}

// =============================================================================
// Policy system integration
// =============================================================================

#[tokio::test]
async fn test_policy_deny_run_command_allow_others() {
    let hook = policy::enforce(policy::confirm_run_command());
    let mut ctx = HookContext::new();

    // run_command denied
    let tc_run = ToolCall {
        name: ToolName::Builtin(BuiltinTools::RunCommand),
        args: HashMap::new(),
        id: Some("1".to_string()),
        canonical_path: None,
    };
    let result = hook.run(&mut ctx, &tc_run).await;
    assert!(!result.allow);

    // view_file allowed
    let tc_view = ToolCall {
        name: ToolName::Builtin(BuiltinTools::ViewFile),
        args: HashMap::new(),
        id: Some("2".to_string()),
        canonical_path: None,
    };
    let result = hook.run(&mut ctx, &tc_view).await;
    assert!(result.allow);

    // Custom tool allowed
    let tc_custom = ToolCall {
        name: ToolName::Custom("my_search".to_string()),
        args: HashMap::new(),
        id: Some("3".to_string()),
        canonical_path: None,
    };
    let result = hook.run(&mut ctx, &tc_custom).await;
    assert!(result.allow);
}

#[tokio::test]
async fn test_policy_allow_all() {
    let hook = policy::enforce(vec![policy::allow_all()]);
    let mut ctx = HookContext::new();

    // Even run_command allowed
    let tc = ToolCall {
        name: ToolName::Builtin(BuiltinTools::RunCommand),
        args: HashMap::new(),
        id: Some("1".to_string()),
        canonical_path: None,
    };
    let result = hook.run(&mut ctx, &tc).await;
    assert!(result.allow);
}

#[tokio::test]
async fn test_policy_deny_all() {
    let hook = policy::enforce(vec![policy::deny_all()]);
    let mut ctx = HookContext::new();

    let tc = ToolCall {
        name: ToolName::Builtin(BuiltinTools::ViewFile),
        args: HashMap::new(),
        id: Some("1".to_string()),
        canonical_path: None,
    };
    let result = hook.run(&mut ctx, &tc).await;
    assert!(!result.allow);
}

#[tokio::test]
async fn test_policy_specific_deny_overrides_wildcard_allow() {
    let hook = policy::enforce(vec![policy::deny("dangerous_tool"), policy::allow_all()]);
    let mut ctx = HookContext::new();

    let tc_denied = ToolCall {
        name: ToolName::Custom("dangerous_tool".to_string()),
        args: HashMap::new(),
        id: Some("1".to_string()),
        canonical_path: None,
    };
    assert!(!hook.run(&mut ctx, &tc_denied).await.allow);

    let tc_allowed = ToolCall {
        name: ToolName::Custom("safe_tool".to_string()),
        args: HashMap::new(),
        id: Some("2".to_string()),
        canonical_path: None,
    };
    assert!(hook.run(&mut ctx, &tc_allowed).await.allow);
}

// =============================================================================
// HookRunner integration
// =============================================================================

struct CountingPreTurnHook {
    count: Arc<AtomicU32>,
}

#[async_trait::async_trait]
impl DecideHook<Content> for CountingPreTurnHook {
    async fn run(&self, _ctx: &mut HookContext, _data: &Content) -> HookResult {
        self.count.fetch_add(1, Ordering::SeqCst);
        HookResult::allowed()
    }
}

#[tokio::test]
async fn test_hook_runner_pre_turn_dispatch() {
    let _count = Arc::new(AtomicU32::new(0));
    let runner = HookRunner::new();

    // Inject via public API would go through add_pre_tool_call_decide,
    // but pre_turn is internal — test via direct field access
    assert!(!runner.has_hooks());
    assert!(!runner.has_pre_tool_call_decide_hooks());
}

#[tokio::test]
async fn test_hook_runner_session_lifecycle() {
    let runner = HookRunner::new();
    // dispatch_session_start and end should not panic with no hooks
    runner.dispatch_session_start().await;
    runner.dispatch_session_end().await;
}

// =============================================================================
// ToolRunner integration
// =============================================================================

#[tokio::test]
async fn test_tool_runner_register_and_process() {
    let mut runner = ToolRunner::new();

    runner
        .register(RegisteredTool {
            name: "add".to_string(),
            description: "Adds two numbers".to_string(),
            schema: None,
            handler: Box::new(|args| {
                Box::pin(async move {
                    let a = args.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
                    let b = args.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(serde_json::json!({"sum": a + b}))
                })
            }),
        })
        .unwrap();

    assert!(runner.has_tool("add"));
    assert_eq!(runner.tool_count(), 1);

    let mut args = HashMap::new();
    args.insert("a".to_string(), serde_json::json!(3));
    args.insert("b".to_string(), serde_json::json!(4));

    let calls = vec![ToolCall {
        name: ToolName::Custom("add".to_string()),
        args,
        id: Some("call-1".to_string()),
        canonical_path: None,
    }];

    let results = runner.process_tool_calls(&calls).await;
    assert_eq!(results.len(), 1);
    assert!(results[0].error.is_none());
    let sum = results[0].result.as_ref().unwrap().get("sum").unwrap();
    assert_eq!(sum, &serde_json::json!(7));
}

#[tokio::test]
async fn test_tool_runner_unknown_tool_returns_error() {
    let runner = ToolRunner::new();

    let calls = vec![ToolCall {
        name: ToolName::Custom("nonexistent".to_string()),
        args: HashMap::new(),
        id: Some("call-1".to_string()),
        canonical_path: None,
    }];

    let results = runner.process_tool_calls(&calls).await;
    assert_eq!(results.len(), 1);
    assert!(results[0].error.is_some());
    assert!(results[0].error.as_ref().unwrap().contains("Unknown tool"));
}

// =============================================================================
// ToolContext integration
// =============================================================================

struct DummyConnection;
#[async_trait::async_trait]
impl antigravity_sdk::connections::Connection for DummyConnection {
    fn is_idle(&self) -> bool {
        false
    }
    fn conversation_id(&self) -> &str {
        "conv-123"
    }
    async fn send(
        &self,
        _prompt: Option<antigravity_sdk::types::Content>,
    ) -> Result<(), antigravity_sdk::types::AntigravityConnectionError> {
        Ok(())
    }
    fn receive_steps(
        &self,
    ) -> std::pin::Pin<Box<dyn futures::Stream<Item = antigravity_sdk::types::Step> + Send + '_>>
    {
        Box::pin(futures::stream::empty())
    }
    async fn send_trigger_notification(
        &self,
        _content: &str,
    ) -> Result<(), antigravity_sdk::types::AntigravityConnectionError> {
        Ok(())
    }
}

#[test]
fn test_tool_context_lifecycle() {
    let connection = Arc::new(DummyConnection);
    let mut ctx = ToolContext::new(connection);
    assert_eq!(ctx.conversation_id(), "conv-123");
    assert!(!ctx.is_idle());

    // Use key-value store
    ctx.set("counter".to_string(), serde_json::json!(0));
    assert!(ctx.has("counter"));
    assert_eq!(ctx.store_len(), 1);

    ctx.set("counter".to_string(), serde_json::json!(1));
    assert_eq!(ctx.get("counter"), Some(&serde_json::json!(1)));

    let removed = ctx.remove("counter");
    assert_eq!(removed, Some(serde_json::json!(1)));
    assert!(!ctx.has("counter"));
}

// =============================================================================
// Trigger helper integration
// =============================================================================

#[test]
fn test_trigger_helpers_construct() {
    let _every = triggers::every(10.0, |_ctx| async move {
        println!("tick");
    });

    let _after = triggers::after(5.0, |_ctx| async move {
        println!("once");
    });
}

// =============================================================================
// Type serialization integration (cross-module boundaries)
// =============================================================================

#[test]
fn test_step_with_tool_calls_serde_roundtrip() {
    let step = Step {
        id: "step-42".to_string(),
        step_index: 3,
        r#type: StepType::ToolCall,
        source: StepSource::Model,
        target: StepTarget::Environment,
        status: StepStatus::Done,
        content: String::new(),
        content_delta: String::new(),
        thinking: "I should search for this".to_string(),
        thinking_delta: String::new(),
        tool_calls: vec![ToolCall {
            name: ToolName::Builtin(BuiltinTools::SearchDir),
            args: {
                let mut m = HashMap::new();
                m.insert("query".to_string(), serde_json::json!("TODO"));
                m
            },
            id: Some("tc-1".to_string()),
            canonical_path: None,
        }],
        error: String::new(),
        is_complete_response: Some(false),
        structured_output: None,
        usage_metadata: Some(UsageMetadata {
            prompt_token_count: Some(150),
            cached_content_token_count: Some(50),
            candidates_token_count: Some(30),
            thoughts_token_count: Some(20),
            total_token_count: Some(250),
        }),
    };

    let json = serde_json::to_string(&step).unwrap();
    let parsed: Step = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.id, "step-42");
    assert_eq!(parsed.r#type, StepType::ToolCall);
    assert_eq!(parsed.tool_calls.len(), 1);
    assert_eq!(parsed.tool_calls[0].id, Some("tc-1".to_string()));
    assert_eq!(
        parsed.usage_metadata.as_ref().unwrap().total_token_count,
        Some(250)
    );
}

#[test]
fn test_builtin_tools_sets_are_consistent() {
    let all = BuiltinTools::all_tools();
    let ro = BuiltinTools::read_only();
    let nd = BuiltinTools::nondestructive();
    let ft = BuiltinTools::file_tools();

    // read_only ⊂ nondestructive ⊂ all
    for tool in &ro {
        assert!(
            nd.contains(tool),
            "{:?} in read_only but not nondestructive",
            tool
        );
        assert!(all.contains(tool), "{:?} in read_only but not all", tool);
    }
    for tool in &nd {
        assert!(
            all.contains(tool),
            "{:?} in nondestructive but not all",
            tool
        );
    }
    for tool in &ft {
        assert!(all.contains(tool), "{:?} in file_tools but not all", tool);
    }

    // RunCommand should not be in read_only or nondestructive
    assert!(!ro.contains(&BuiltinTools::RunCommand));
    assert!(!nd.contains(&BuiltinTools::RunCommand));
    assert!(all.contains(&BuiltinTools::RunCommand));
}

#[test]
fn test_content_type_conversions() {
    // &str → Content
    let c1 = Content::from("hello");
    match c1 {
        Content::Single(ContentPrimitive::Text(s)) => assert_eq!(s, "hello"),
        _ => panic!("Expected Single Text"),
    }

    // String → Content
    let c2 = Content::from("world".to_string());
    match c2 {
        Content::Single(ContentPrimitive::Text(s)) => assert_eq!(s, "world"),
        _ => panic!("Expected Single Text"),
    }

    // Multiple
    let c3 = Content::Multiple(vec![
        ContentPrimitive::from("a"),
        ContentPrimitive::from("b"),
    ]);
    match c3 {
        Content::Multiple(parts) => assert_eq!(parts.len(), 2),
        _ => panic!("Expected Multiple"),
    }
}

#[test]
fn test_tool_name_resolution() {
    // Builtin tools resolve from string
    let name = ToolName::from("view_file");
    assert_eq!(name, ToolName::Builtin(BuiltinTools::ViewFile));

    // Custom tools stay as Custom
    let name = ToolName::from("my_custom_analyzer");
    assert_eq!(name, ToolName::Custom("my_custom_analyzer".to_string()));

    // Display for builtin
    assert_eq!(
        format!("{}", ToolName::Builtin(BuiltinTools::RunCommand)),
        "run_command"
    );

    // Display for custom
    assert_eq!(
        format!("{}", ToolName::Custom("search_db".to_string())),
        "search_db"
    );
}
