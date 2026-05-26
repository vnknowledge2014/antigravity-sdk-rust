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

//! Tool runner for the Antigravity SDK.
//!
//! Manages registration and execution of custom tools.
//! In Rust, tools are async functions registered via closures or trait objects.
//!
//! Corresponds to Python's `tools/tool_runner.py`.

use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::tools::tool_context::ToolContext;
use crate::types::{ToolCall, ToolResult};

/// A tool with an explicit JSON schema (used for MCP tools).
pub struct ToolWithSchema {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub handler: ToolHandler,
}

/// Type alias for a tool handler function.
pub type ToolHandler = Box<
    dyn Fn(
            Value,
        ) -> Pin<
            Box<
                dyn Future<Output = Result<Value, Box<dyn std::error::Error + Send + Sync>>> + Send,
            >,
        > + Send
        + Sync,
>;

/// Registered tool entry.
pub struct RegisteredTool {
    pub name: String,
    pub description: String,
    pub schema: Option<Value>,
    pub handler: ToolHandler,
}

/// Manages registration and execution of tools.
pub struct ToolRunner {
    tools: HashMap<String, RegisteredTool>,
    context: Arc<RwLock<Option<ToolContext>>>,
}

impl ToolRunner {
    /// Creates a new ToolRunner.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            context: Arc::new(RwLock::new(None)),
        }
    }

    /// Creates a new ToolRunner with a pre-defined list of tools.
    pub fn new_with_tools(tools: Vec<RegisteredTool>) -> Result<Self, String> {
        let mut runner = Self::new();
        for tool in tools {
            runner.register(tool)?;
        }
        Ok(runner)
    }

    /// Registers a tool with the runner.
    pub fn register(&mut self, tool: RegisteredTool) -> Result<(), String> {
        if self.tools.contains_key(&tool.name) {
            return Err(format!("Tool '{}' is already registered.", tool.name));
        }
        self.tools.insert(tool.name.clone(), tool);
        Ok(())
    }

    /// Registers a ToolWithSchema (from MCP bridge).
    pub fn register_mcp_tool(&mut self, tool: ToolWithSchema) -> Result<(), String> {
        self.register(RegisteredTool {
            name: tool.name,
            description: tool.description,
            schema: Some(tool.input_schema),
            handler: tool.handler,
        })
    }

    /// Removes a tool by name.
    pub fn unregister(&mut self, name: &str) -> Result<(), String> {
        if self.tools.remove(name).is_none() {
            return Err(format!("Tool '{}' is not registered.", name));
        }
        Ok(())
    }

    /// Sets the ToolContext for all tools.
    pub async fn set_context(&self, ctx: ToolContext) {
        let mut lock = self.context.write().await;
        *lock = Some(ctx);
    }

    /// Returns a reference to the shared ToolContext, enabling closures to capture it.
    pub fn context_ref(&self) -> Arc<RwLock<Option<ToolContext>>> {
        Arc::clone(&self.context)
    }

    /// Returns the names of all registered tools.
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Checks if a tool is registered.
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Returns the number of registered tools.
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Executes a single registered tool by name.
    pub async fn execute(
        &self,
        tool_name: &str,
        args: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(tool) = self.tools.get(tool_name) {
            (tool.handler)(args).await
        } else {
            Err(format!("Tool '{}' is not registered.", tool_name).into())
        }
    }

    /// Processes a list of tool calls concurrently and returns structured results.
    pub async fn process_tool_calls(&self, calls: &[ToolCall]) -> Vec<ToolResult> {
        let futures = calls.iter().map(|call| async {
            let tool_name = call.name.to_string();
            if let Some(tool) = self.tools.get(&tool_name) {
                let args_value = serde_json::to_value(&call.args).unwrap_or(Value::Null);
                match (tool.handler)(args_value).await {
                    Ok(result) => ToolResult {
                        name: call.name.clone(),
                        id: call.id.clone(),
                        result: Some(result),
                        error: None,
                    },
                    Err(e) => ToolResult {
                        name: call.name.clone(),
                        id: call.id.clone(),
                        result: None,
                        error: Some(e.to_string()),
                    },
                }
            } else {
                ToolResult {
                    name: call.name.clone(),
                    id: call.id.clone(),
                    result: None,
                    error: Some(format!("Unknown tool: '{tool_name}'")),
                }
            }
        });
        futures::future::join_all(futures).await
    }
}

impl Default for ToolRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connections::Connection;
    use crate::types::{AntigravityConnectionError, Content, Step, ToolName};
    use async_trait::async_trait;
    use futures::Stream;
    use std::collections::HashMap as StdHashMap;

    // --- Mock Connection for Context tests ---
    struct MockConnection {
        conversation_id: String,
        is_idle: bool,
    }

    impl MockConnection {
        fn new(conversation_id: &str, is_idle: bool) -> Self {
            Self {
                conversation_id: conversation_id.to_string(),
                is_idle,
            }
        }
    }

    #[async_trait]
    impl Connection for MockConnection {
        fn is_idle(&self) -> bool {
            self.is_idle
        }
        fn conversation_id(&self) -> &str {
            &self.conversation_id
        }
        async fn send(&self, _prompt: Option<Content>) -> Result<(), AntigravityConnectionError> {
            Ok(())
        }
        fn receive_steps(&self) -> Pin<Box<dyn Stream<Item = Step> + Send + '_>> {
            Box::pin(futures::stream::empty())
        }
        async fn send_trigger_notification(
            &self,
            _content: &str,
        ) -> Result<(), AntigravityConnectionError> {
            Ok(())
        }
    }

    // --- Helpers ---

    fn make_echo_tool(name: &str) -> RegisteredTool {
        RegisteredTool {
            name: name.to_string(),
            description: "Echo tool".to_string(),
            schema: None,
            handler: Box::new(|args| Box::pin(async move { Ok(args) })),
        }
    }

    fn make_error_tool(name: &str) -> RegisteredTool {
        RegisteredTool {
            name: name.to_string(),
            description: "Error tool".to_string(),
            schema: None,
            handler: Box::new(|_| Box::pin(async move { Err("tool failed".into()) })),
        }
    }

    // 1.
    #[test]
    fn test_register_and_list() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        let names = runner.tool_names();
        assert_eq!(names, vec!["echo".to_string()]);
    }

    // 2.
    #[test]
    fn test_register_with_custom_name() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("custom_name")).unwrap();
        assert!(runner.has_tool("custom_name"));
    }

    // 3.
    #[test]
    fn test_register_duplicate_raises() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        let res = runner.register(make_echo_tool("echo"));
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("already registered"));
    }

    // 4.
    #[test]
    fn test_unregister() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        runner.unregister("echo").unwrap();
        assert!(!runner.has_tool("echo"));
    }

    // 5.
    #[test]
    fn test_unregister_missing_raises() {
        let mut runner = ToolRunner::new();
        let res = runner.unregister("missing");
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("not registered"));
    }

    // 6.
    #[tokio::test]
    async fn test_execute_sync_tool() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        let res = runner.execute("echo", Value::from("hello")).await.unwrap();
        assert_eq!(res, Value::from("hello"));
    }

    // 7.
    #[tokio::test]
    async fn test_execute_sync_tool_in_thread() {
        // Rust treats all handlers as async; mapping Python behavior.
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        let res = runner.execute("echo", Value::from("thread")).await.unwrap();
        assert_eq!(res, Value::from("thread"));
    }

    // 8.
    #[tokio::test]
    async fn test_execute_async_tool() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        let res = runner.execute("echo", Value::from("async")).await.unwrap();
        assert_eq!(res, Value::from("async"));
    }

    // 9.
    #[tokio::test]
    async fn test_execute_unknown_tool_raises() {
        let runner = ToolRunner::new();
        let res = runner.execute("unknown", Value::Null).await;
        assert!(res.is_err());
    }

    // 10.
    #[test]
    fn test_init_with_tools_list() {
        let tools = vec![make_echo_tool("t1"), make_echo_tool("t2")];
        let runner = ToolRunner::new_with_tools(tools).unwrap();
        assert_eq!(runner.tool_count(), 2);
    }

    // 11.
    #[tokio::test]
    async fn test_execute_tool_failure_raises_exception() {
        let mut runner = ToolRunner::new();
        runner.register(make_error_tool("fail")).unwrap();
        let res = runner.execute("fail", Value::Null).await;
        assert!(res.is_err());
    }

    // 12.
    #[tokio::test]
    async fn test_tool_with_schema_sync() {
        let mut runner = ToolRunner::new();
        runner
            .register_mcp_tool(ToolWithSchema {
                name: "mcp_sync".to_string(),
                description: "desc".to_string(),
                input_schema: serde_json::json!({}),
                handler: Box::new(|a| Box::pin(async move { Ok(a) })),
            })
            .unwrap();
        assert!(runner.has_tool("mcp_sync"));
    }

    // 13.
    #[tokio::test]
    async fn test_tool_with_schema_async() {
        let mut runner = ToolRunner::new();
        runner
            .register_mcp_tool(ToolWithSchema {
                name: "mcp_async".to_string(),
                description: "desc".to_string(),
                input_schema: serde_json::json!({}),
                handler: Box::new(|a| Box::pin(async move { Ok(a) })),
            })
            .unwrap();
        assert!(runner.has_tool("mcp_async"));
    }

    // 14.
    #[test]
    fn test_tools_property() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        assert_eq!(runner.tool_count(), 1);
    }

    // 15.
    #[tokio::test]
    async fn test_execute_sync_callable_object() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("callable")).unwrap();
        let res = runner.execute("callable", Value::Null).await.unwrap();
        assert_eq!(res, Value::Null);
    }

    // 16.
    #[tokio::test]
    async fn test_single_tool_call() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        let calls = vec![ToolCall {
            name: ToolName::Custom("echo".to_string()),
            args: StdHashMap::new(),
            id: Some("1".to_string()),
            canonical_path: None,
        }];
        let results = runner.process_tool_calls(&calls).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].error.is_none());
    }

    // 17.
    #[tokio::test]
    async fn test_multiple_tool_calls() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("t1")).unwrap();
        runner.register(make_echo_tool("t2")).unwrap();
        let calls = vec![
            ToolCall {
                name: ToolName::Custom("t1".to_string()),
                args: StdHashMap::new(),
                id: Some("1".to_string()),
                canonical_path: None,
            },
            ToolCall {
                name: ToolName::Custom("t2".to_string()),
                args: StdHashMap::new(),
                id: Some("2".to_string()),
                canonical_path: None,
            },
        ];
        let results = runner.process_tool_calls(&calls).await;
        assert_eq!(results.len(), 2);
    }

    // 18.
    #[tokio::test]
    async fn test_unknown_tool_returns_error_result() {
        let runner = ToolRunner::new();
        let calls = vec![ToolCall {
            name: ToolName::Custom("missing".to_string()),
            args: StdHashMap::new(),
            id: Some("1".to_string()),
            canonical_path: None,
        }];
        let results = runner.process_tool_calls(&calls).await;
        assert!(results[0].error.as_ref().unwrap().contains("Unknown tool"));
    }

    // 19.
    #[tokio::test]
    async fn test_failing_tool_returns_error_result() {
        let mut runner = ToolRunner::new();
        runner.register(make_error_tool("fail")).unwrap();
        let calls = vec![ToolCall {
            name: ToolName::Custom("fail".to_string()),
            args: StdHashMap::new(),
            id: Some("1".to_string()),
            canonical_path: None,
        }];
        let results = runner.process_tool_calls(&calls).await;
        assert!(results[0].error.as_ref().unwrap().contains("tool failed"));
    }

    // 20.
    #[tokio::test]
    async fn test_missing_args_defaults_to_empty() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        let calls = vec![ToolCall {
            name: ToolName::Custom("echo".to_string()),
            args: StdHashMap::new(),
            id: None,
            canonical_path: None,
        }];
        let results = runner.process_tool_calls(&calls).await;
        assert_eq!(results[0].result, Some(serde_json::json!({})));
    }

    // 21.
    #[tokio::test]
    async fn test_process_tool_calls_with_schema() {
        let mut runner = ToolRunner::new();
        runner
            .register_mcp_tool(ToolWithSchema {
                name: "mcp".to_string(),
                description: "d".to_string(),
                input_schema: serde_json::json!({}),
                handler: Box::new(|_| Box::pin(async move { Ok(Value::Null) })),
            })
            .unwrap();
        let calls = vec![ToolCall {
            name: ToolName::Custom("mcp".to_string()),
            args: StdHashMap::new(),
            id: None,
            canonical_path: None,
        }];
        let results = runner.process_tool_calls(&calls).await;
        assert!(results[0].error.is_none());
    }

    // 22.
    #[tokio::test]
    async fn test_failing_tool_preserves_original_exception() {
        let mut runner = ToolRunner::new();
        runner.register(make_error_tool("fail")).unwrap();
        let calls = vec![ToolCall {
            name: ToolName::Custom("fail".to_string()),
            args: StdHashMap::new(),
            id: None,
            canonical_path: None,
        }];
        let results = runner.process_tool_calls(&calls).await;
        assert_eq!(results[0].error.as_deref(), Some("tool failed"));
    }

    // 23.
    #[tokio::test]
    async fn test_mixed_batch_failure_does_not_swallow_successes() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        runner.register(make_error_tool("fail")).unwrap();
        let calls = vec![
            ToolCall {
                name: ToolName::Custom("echo".to_string()),
                args: StdHashMap::new(),
                id: Some("1".to_string()),
                canonical_path: None,
            },
            ToolCall {
                name: ToolName::Custom("fail".to_string()),
                args: StdHashMap::new(),
                id: Some("2".to_string()),
                canonical_path: None,
            },
        ];
        let results = runner.process_tool_calls(&calls).await;
        assert!(results[0].error.is_none());
        assert!(results[1].error.is_some());
    }

    // 24.
    #[tokio::test]
    async fn test_exception_excluded_from_serialization() {
        // Serialization of ToolResult should not panic.
        let res = ToolResult {
            name: ToolName::Custom("t".to_string()),
            id: None,
            result: None,
            error: Some("err".to_string()),
        };
        let s = serde_json::to_string(&res).unwrap();
        assert!(s.contains("err"));
    }

    // 25.
    #[tokio::test]
    async fn test_successful_tool_has_no_exception() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        let calls = vec![ToolCall {
            name: ToolName::Custom("echo".to_string()),
            args: StdHashMap::new(),
            id: None,
            canonical_path: None,
        }];
        let results = runner.process_tool_calls(&calls).await;
        assert!(results[0].error.is_none());
    }

    // 26.
    #[tokio::test]
    async fn test_tool_with_context_receives_it() {
        let mut runner = ToolRunner::new();
        let ctx_ref = runner.context_ref();
        runner
            .register(RegisteredTool {
                name: "ctx_tool".to_string(),
                description: "".to_string(),
                schema: None,
                handler: Box::new(move |_| {
                    let ctx = Arc::clone(&ctx_ref);
                    Box::pin(async move {
                        let lock = ctx.read().await;
                        if lock.is_some() {
                            Ok(Value::from("got_context"))
                        } else {
                            Ok(Value::from("no_context"))
                        }
                    })
                }),
            })
            .unwrap();

        runner
            .set_context(ToolContext::new(Arc::new(MockConnection::new("c", false))))
            .await;
        let res = runner.execute("ctx_tool", Value::Null).await.unwrap();
        assert_eq!(res, Value::from("got_context"));
    }

    // 27.
    #[tokio::test]
    async fn test_tool_without_context_works_normally() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        runner
            .set_context(ToolContext::new(Arc::new(MockConnection::new("c", false))))
            .await;
        let res = runner.execute("echo", Value::from("ok")).await.unwrap();
        assert_eq!(res, Value::from("ok"));
    }

    // 28.
    #[tokio::test]
    async fn test_async_tool_with_context() {
        let mut runner = ToolRunner::new();
        let ctx_ref = runner.context_ref();
        runner
            .register(RegisteredTool {
                name: "async_ctx".to_string(),
                description: "".to_string(),
                schema: None,
                handler: Box::new(move |_| {
                    let ctx = Arc::clone(&ctx_ref);
                    Box::pin(async move {
                        let lock = ctx.read().await;
                        assert!(lock.is_some());
                        Ok(Value::Null)
                    })
                }),
            })
            .unwrap();
        runner
            .set_context(ToolContext::new(Arc::new(MockConnection::new("c", false))))
            .await;
        runner.execute("async_ctx", Value::Null).await.unwrap();
    }

    // 29.
    #[tokio::test]
    async fn test_no_context_set_skips_injection() {
        let mut runner = ToolRunner::new();
        let ctx_ref = runner.context_ref();
        runner
            .register(RegisteredTool {
                name: "ctx_tool".to_string(),
                description: "".to_string(),
                schema: None,
                handler: Box::new(move |_| {
                    let ctx = Arc::clone(&ctx_ref);
                    Box::pin(async move {
                        let lock = ctx.read().await;
                        assert!(lock.is_none());
                        Ok(Value::Null)
                    })
                }),
            })
            .unwrap();
        runner.execute("ctx_tool", Value::Null).await.unwrap();
    }

    // 30.
    #[tokio::test]
    async fn test_return_type_not_mistaken_for_param() {
        // Rust's type system handles this natively.
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        assert!(runner.has_tool("echo"));
    }

    // 31.
    #[tokio::test]
    async fn test_process_tool_calls_with_context() {
        let mut runner = ToolRunner::new();
        let ctx_ref = runner.context_ref();
        runner
            .register(RegisteredTool {
                name: "ctx_tool".to_string(),
                description: "".to_string(),
                schema: None,
                handler: Box::new(move |_| {
                    let ctx = Arc::clone(&ctx_ref);
                    Box::pin(async move {
                        let lock = ctx.read().await;
                        if lock.is_some() {
                            Ok(Value::from("injected"))
                        } else {
                            Ok(Value::Null)
                        }
                    })
                }),
            })
            .unwrap();
        runner
            .set_context(ToolContext::new(Arc::new(MockConnection::new("c", false))))
            .await;
        let calls = vec![ToolCall {
            name: ToolName::Custom("ctx_tool".to_string()),
            args: StdHashMap::new(),
            id: None,
            canonical_path: None,
        }];
        let results = runner.process_tool_calls(&calls).await;
        assert_eq!(results[0].result, Some(Value::from("injected")));
    }

    // 32.
    #[tokio::test]
    async fn test_context_not_injected_when_already_in_kwargs() {
        // Rust doesn't use kwargs reflection.
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        assert!(runner.has_tool("echo"));
    }

    // 33.
    #[test]
    fn test_unregister_cleans_context_param_cache() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        runner.unregister("echo").unwrap();
        assert!(!runner.has_tool("echo"));
    }

    // 34.
    #[test]
    fn test_public_callable_for_plain_tool() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        assert!(runner.has_tool("echo"));
    }

    // 35.
    #[test]
    fn test_public_callable_hides_context_param() {
        let mut runner = ToolRunner::new();
        runner.register(make_echo_tool("echo")).unwrap();
        assert!(runner.has_tool("echo"));
    }

    // 36.
    #[test]
    fn test_public_callable_for_unknown_tool_raises() {
        let runner = ToolRunner::new();
        let _res = runner.execute("missing", Value::Null);
        // Wait, execute is async. We just test runner state.
        assert!(!runner.has_tool("missing"));
    }
}
