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

//! HookRunner — manages and dispatches hooks.
//!
//! Corresponds to Python's `hooks/hook_runner.py`.

use crate::hooks::hooks::*;
use crate::types::{
    AskQuestionInteractionSpec, Content, HookResult, QuestionHookResult, Step, ToolCall, ToolResult,
};
use std::sync::{Arc, RwLock};

pub enum AnyHook {
    OnSessionStart(Box<OnSessionStartHook>),
    OnSessionEnd(Box<OnSessionEndHook>),
    PreTurn(Box<PreTurnHook>),
    PostTurn(Box<PostTurnHook>),
    PreToolCallDecide(Box<PreToolCallDecideHook>),
    PostToolCall(Box<PostToolCallHook>),
    OnToolError(Box<OnToolErrorHook>),
    OnInteraction(Box<OnInteractionHook>),
    OnCompaction(Box<OnCompactionHook>),
}

/// Manages collections of specific hook types and dispatches events.
pub struct HookRunner {
    pub session_context: Arc<RwLock<SessionContext>>,
    on_session_start_hooks: Vec<Box<OnSessionStartHook>>,
    on_session_end_hooks: Vec<Box<OnSessionEndHook>>,
    pre_turn_hooks: Vec<Box<PreTurnHook>>,
    post_turn_hooks: Vec<Box<PostTurnHook>>,
    pre_tool_call_decide_hooks: Vec<Box<PreToolCallDecideHook>>,
    post_tool_call_hooks: Vec<Box<PostToolCallHook>>,
    on_tool_error_hooks: Vec<Box<OnToolErrorHook>>,
    on_interaction_hooks: Vec<Box<OnInteractionHook>>,
    on_compaction_hooks: Vec<Box<OnCompactionHook>>,
}

impl HookRunner {
    pub fn new() -> Self {
        Self {
            session_context: Arc::new(RwLock::new(SessionContext::default())),
            on_session_start_hooks: Vec::new(),
            on_session_end_hooks: Vec::new(),
            pre_turn_hooks: Vec::new(),
            post_turn_hooks: Vec::new(),
            pre_tool_call_decide_hooks: Vec::new(),
            post_tool_call_hooks: Vec::new(),
            on_tool_error_hooks: Vec::new(),
            on_interaction_hooks: Vec::new(),
            on_compaction_hooks: Vec::new(),
        }
    }

    pub fn has_hooks(&self) -> bool {
        !self.on_session_start_hooks.is_empty()
            || !self.on_session_end_hooks.is_empty()
            || !self.pre_turn_hooks.is_empty()
            || !self.post_turn_hooks.is_empty()
            || !self.pre_tool_call_decide_hooks.is_empty()
            || !self.post_tool_call_hooks.is_empty()
            || !self.on_tool_error_hooks.is_empty()
            || !self.on_interaction_hooks.is_empty()
            || !self.on_compaction_hooks.is_empty()
    }

    pub fn has_pre_tool_call_decide_hooks(&self) -> bool {
        !self.pre_tool_call_decide_hooks.is_empty()
    }

    pub fn clear_pre_tool_call_decide_hooks(&mut self) {
        self.pre_tool_call_decide_hooks.clear();
    }

    pub fn pre_tool_call_decide_mut(&mut self) -> &mut Vec<Box<PreToolCallDecideHook>> {
        &mut self.pre_tool_call_decide_hooks
    }

    pub fn register_hook(&mut self, hook: AnyHook) {
        match hook {
            AnyHook::OnSessionStart(h) => self.on_session_start_hooks.push(h),
            AnyHook::OnSessionEnd(h) => self.on_session_end_hooks.push(h),
            AnyHook::PreTurn(h) => self.pre_turn_hooks.push(h),
            AnyHook::PostTurn(h) => self.post_turn_hooks.push(h),
            AnyHook::PreToolCallDecide(h) => self.pre_tool_call_decide_hooks.push(h),
            AnyHook::PostToolCall(h) => self.post_tool_call_hooks.push(h),
            AnyHook::OnToolError(h) => self.on_tool_error_hooks.push(h),
            AnyHook::OnInteraction(h) => self.on_interaction_hooks.push(h),
            AnyHook::OnCompaction(h) => self.on_compaction_hooks.push(h),
        }
    }

    // --- Dispatch methods ---

    pub async fn dispatch_session_start(&self) {
        for hook in &self.on_session_start_hooks {
            let mut ctx = HookContext::new();
            hook.run(&mut ctx, &()).await;
        }
    }

    pub async fn dispatch_session_end(&self) {
        for hook in &self.on_session_end_hooks {
            let mut ctx = HookContext::new();
            hook.run(&mut ctx, &()).await;
        }
    }

    pub async fn dispatch_pre_turn(&self, prompt: Option<&Content>) -> (HookResult, TurnContext) {
        let mut turn_context = TurnContext::new(&self.session_context.read().unwrap());
        let default_content = Content::Single(crate::types::ContentPrimitive::Text("".to_string()));
        let prompt_ref = prompt.unwrap_or(&default_content);
        for hook in &self.pre_turn_hooks {
            let res = hook.run(&mut turn_context.ctx, prompt_ref).await;
            if !res.allow {
                return (res, turn_context);
            }
        }
        (HookResult::allowed(), turn_context)
    }

    pub async fn dispatch_post_turn(&self, turn_ctx: &mut TurnContext, response: &str) {
        let owned = response.to_string();
        for hook in &self.post_turn_hooks {
            hook.run(&mut turn_ctx.ctx, &owned).await;
        }
    }

    pub async fn dispatch_pre_tool_call(
        &self,
        turn_ctx: &TurnContext,
        tool_call: &ToolCall,
    ) -> (HookResult, OperationContext) {
        let mut op_context = OperationContext::new(turn_ctx);
        for hook in &self.pre_tool_call_decide_hooks {
            let res = hook.run(&mut op_context.ctx, tool_call).await;
            if !res.allow {
                return (res, op_context);
            }
        }
        (HookResult::allowed(), op_context)
    }

    pub async fn dispatch_post_tool_call(
        &self,
        op_ctx: &mut OperationContext,
        result: &ToolResult,
    ) {
        for hook in &self.post_tool_call_hooks {
            hook.run(&mut op_ctx.ctx, result).await;
        }
    }

    pub async fn dispatch_on_tool_error(
        &self,
        op_ctx: &mut OperationContext,
        error: Box<dyn std::error::Error + Send + Sync>,
    ) -> (HookResult, Option<serde_json::Value>) {
        for hook in &self.on_tool_error_hooks {
            let res = hook.run(&mut op_ctx.ctx, &error).await;
            if res.is_some() {
                return (HookResult::allowed(), res);
            }
        }
        (HookResult::denied("Error"), None)
    }

    pub async fn dispatch_interaction(
        &self,
        turn_ctx: &TurnContext,
        interaction_spec: &AskQuestionInteractionSpec,
    ) -> (HookResult, Option<QuestionHookResult>, OperationContext) {
        let mut op_context = OperationContext::new(turn_ctx);
        for hook in &self.on_interaction_hooks {
            let res = hook.run(&mut op_context.ctx, interaction_spec).await;
            if res.is_some() {
                return (HookResult::allowed(), res, op_context);
            }
        }
        (
            HookResult::denied("No interaction hook handled the request"),
            None,
            op_context,
        )
    }

    pub async fn dispatch_question(
        &self,
        turn_ctx: &TurnContext,
        question: &AskQuestionInteractionSpec,
    ) -> (HookResult, Option<QuestionHookResult>, OperationContext) {
        self.dispatch_interaction(turn_ctx, question).await
    }

    pub async fn dispatch_compaction(&self, turn_ctx: &TurnContext, data: &Step) {
        let mut op_context = OperationContext::new(turn_ctx);
        for hook in &self.on_compaction_hooks {
            hook.run(&mut op_context.ctx, data).await;
        }
    }
}

impl Default for HookRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ContentPrimitive, QuestionResponse, ToolName};
    use async_trait::async_trait;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    struct DummyPreTurnHook;
    #[async_trait]
    impl DecideHook<Content> for DummyPreTurnHook {
        async fn run(&self, _ctx: &mut HookContext, _data: &Content) -> HookResult {
            HookResult::allowed()
        }
    }

    struct DenyPreTurnHook;
    #[async_trait]
    impl DecideHook<Content> for DenyPreTurnHook {
        async fn run(&self, _ctx: &mut HookContext, _data: &Content) -> HookResult {
            HookResult::denied("Denied")
        }
    }

    #[tokio::test]
    async fn test_dispatch_pre_turn_allow() {
        let mut runner = HookRunner::new();
        runner.register_hook(AnyHook::PreTurn(Box::new(DummyPreTurnHook)));
        let content = Content::Single(ContentPrimitive::Text("prompt".to_string()));
        let (res, _turn_context) = runner.dispatch_pre_turn(Some(&content)).await;
        assert!(res.allow);
    }

    #[tokio::test]
    async fn test_dispatch_pre_turn_deny() {
        let mut runner = HookRunner::new();
        runner.register_hook(AnyHook::PreTurn(Box::new(DenyPreTurnHook)));
        let content = Content::Single(ContentPrimitive::Text("prompt".to_string()));
        let (res, _turn_context) = runner.dispatch_pre_turn(Some(&content)).await;
        assert!(!res.allow);
        assert_eq!(res.message, "Denied");
    }

    struct CapturePreTurnHook {
        captured: Arc<RwLock<Vec<String>>>,
    }
    #[async_trait]
    impl DecideHook<Content> for CapturePreTurnHook {
        async fn run(&self, _ctx: &mut HookContext, data: &Content) -> HookResult {
            if let Content::Single(ContentPrimitive::Text(s)) = data {
                self.captured.write().unwrap().push(s.clone());
            }
            HookResult::allowed()
        }
    }

    #[tokio::test]
    async fn test_dispatch_pre_turn_none_normalizes_to_empty_string() {
        let captured = Arc::new(RwLock::new(Vec::new()));
        let mut runner = HookRunner::new();
        runner.register_hook(AnyHook::PreTurn(Box::new(CapturePreTurnHook {
            captured: captured.clone(),
        })));

        let (res, _) = runner.dispatch_pre_turn(None).await;
        assert!(res.allow);
        assert_eq!(captured.read().unwrap()[0], "");
    }

    struct CallTrackingHook {
        called: Arc<AtomicBool>,
    }
    #[async_trait]
    impl InspectHook<()> for CallTrackingHook {
        async fn run(&self, _ctx: &mut HookContext, _data: &()) {
            self.called.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_dispatch_session_start() {
        let called = Arc::new(AtomicBool::new(false));
        let mut runner = HookRunner::new();
        runner.register_hook(AnyHook::OnSessionStart(Box::new(CallTrackingHook {
            called: called.clone(),
        })));
        runner.dispatch_session_start().await;
        assert!(called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_dispatch_session_end() {
        let called = Arc::new(AtomicBool::new(false));
        let mut runner = HookRunner::new();
        runner.register_hook(AnyHook::OnSessionEnd(Box::new(CallTrackingHook {
            called: called.clone(),
        })));
        runner.dispatch_session_end().await;
        assert!(called.load(Ordering::SeqCst));
    }

    struct InteractionHook;
    #[async_trait]
    impl TransformHook<AskQuestionInteractionSpec, Option<QuestionHookResult>> for InteractionHook {
        async fn run(
            &self,
            _ctx: &mut HookContext,
            data: &AskQuestionInteractionSpec,
        ) -> Option<QuestionHookResult> {
            if !data.questions.is_empty() && data.questions[0].question == "magic_question" {
                Some(QuestionHookResult {
                    responses: vec![QuestionResponse {
                        freeform_response: "magic_answer".to_string(),
                        ..Default::default()
                    }],
                    cancelled: false,
                })
            } else {
                None
            }
        }
    }

    #[tokio::test]
    async fn test_dispatch_interaction() {
        let mut runner = HookRunner::new();
        runner.register_hook(AnyHook::OnInteraction(Box::new(InteractionHook)));
        let turn_ctx = TurnContext::new(&runner.session_context.read().unwrap());

        let magic_spec = AskQuestionInteractionSpec {
            questions: vec![crate::types::AskQuestionEntry {
                question: "magic_question".to_string(),
                options: vec![],
                is_multi_select: false,
            }],
        };
        let (res, answer, _) = runner.dispatch_interaction(&turn_ctx, &magic_spec).await;
        assert!(res.allow);
        assert_eq!(
            answer.unwrap().responses[0].freeform_response,
            "magic_answer"
        );

        let other_spec = AskQuestionInteractionSpec {
            questions: vec![crate::types::AskQuestionEntry {
                question: "other".to_string(),
                options: vec![],
                is_multi_select: false,
            }],
        };
        let (res2, answer2, _) = runner.dispatch_interaction(&turn_ctx, &other_spec).await;
        assert!(!res2.allow);
        assert!(answer2.is_none());
    }

    struct OrderDecideHook {
        calls: Arc<RwLock<Vec<String>>>,
    }
    #[async_trait]
    impl DecideHook<ToolCall> for OrderDecideHook {
        async fn run(&self, _ctx: &mut HookContext, _data: &ToolCall) -> HookResult {
            self.calls.write().unwrap().push("decide".to_string());
            HookResult::allowed()
        }
    }

    #[tokio::test]
    async fn test_dispatch_pre_tool_call_decide() {
        let calls = Arc::new(RwLock::new(Vec::new()));
        let mut runner = HookRunner::new();
        runner.register_hook(AnyHook::PreToolCallDecide(Box::new(OrderDecideHook {
            calls: calls.clone(),
        })));

        let turn_ctx = TurnContext::new(&runner.session_context.read().unwrap());
        let tool_call = ToolCall {
            name: ToolName::Custom("t".to_string()),
            args: Default::default(),
            id: None,
            canonical_path: None,
        };

        let (res, _) = runner.dispatch_pre_tool_call(&turn_ctx, &tool_call).await;
        assert!(res.allow);
        assert_eq!(calls.read().unwrap()[0], "decide");
    }

    #[tokio::test]
    async fn test_context_scoping() {
        let runner = HookRunner::new();
        runner
            .session_context
            .write()
            .unwrap()
            .ctx
            .set("session_key".to_string(), "session_value".to_string());

        let mut turn_ctx = TurnContext::new(&runner.session_context.read().unwrap());
        turn_ctx
            .ctx
            .set("turn_key".to_string(), "turn_value".to_string());

        let mut op_ctx = OperationContext::new(&turn_ctx);
        op_ctx.ctx.set("op_key".to_string(), "op_value".to_string());

        assert_eq!(
            op_ctx.ctx.get::<String>("op_key"),
            Some(&"op_value".to_string())
        );
        assert_eq!(
            turn_ctx.ctx.get::<String>("turn_key"),
            Some(&"turn_value".to_string())
        );
        assert_eq!(
            runner
                .session_context
                .read()
                .unwrap()
                .ctx
                .get::<String>("session_key"),
            Some(&"session_value".to_string())
        );

        assert_eq!(turn_ctx.ctx.get::<String>("op_key"), None);
        assert_eq!(
            runner
                .session_context
                .read()
                .unwrap()
                .ctx
                .get::<String>("turn_key"),
            None
        );
    }

    struct RecoverErrorHook;
    #[async_trait]
    impl TransformHook<Box<dyn std::error::Error + Send + Sync>, Option<serde_json::Value>>
        for RecoverErrorHook
    {
        async fn run(
            &self,
            _ctx: &mut HookContext,
            _data: &Box<dyn std::error::Error + Send + Sync>,
        ) -> Option<serde_json::Value> {
            Some(serde_json::Value::String("recovered_result".to_string()))
        }
    }

    #[tokio::test]
    async fn test_dispatch_on_tool_error_recovery() {
        let mut runner = HookRunner::new();
        runner.register_hook(AnyHook::OnToolError(Box::new(RecoverErrorHook)));
        let turn_ctx = TurnContext::new(&runner.session_context.read().unwrap());
        let mut op_ctx = OperationContext::new(&turn_ctx);

        let error: Box<dyn std::error::Error + Send + Sync> =
            Box::new(crate::types::AntigravityConnectionError {
                message: "Error".to_string(),
            });
        let (res, data) = runner.dispatch_on_tool_error(&mut op_ctx, error).await;

        assert!(res.allow);
        assert_eq!(
            data,
            Some(serde_json::Value::String("recovered_result".to_string()))
        );
    }

    struct CaptureCompactionHook {
        calls: Arc<AtomicU32>,
    }
    #[async_trait]
    impl InspectHook<Step> for CaptureCompactionHook {
        async fn run(&self, _ctx: &mut HookContext, _data: &Step) {
            self.calls.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_dispatch_compaction() {
        let calls = Arc::new(AtomicU32::new(0));
        let mut runner = HookRunner::new();
        runner.register_hook(AnyHook::OnCompaction(Box::new(CaptureCompactionHook {
            calls: calls.clone(),
        })));

        let turn_ctx = TurnContext::new(&runner.session_context.read().unwrap());
        runner
            .dispatch_compaction(&turn_ctx, &Step::default())
            .await;

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_has_hooks_includes_compaction() {
        let mut runner = HookRunner::new();
        assert!(!runner.has_hooks());
        runner.register_hook(AnyHook::OnCompaction(Box::new(CaptureCompactionHook {
            calls: Arc::new(AtomicU32::new(0)),
        })));
        assert!(runner.has_hooks());
    }

    #[tokio::test]
    async fn test_register_hook() {
        let mut runner = HookRunner::new();
        runner.register_hook(AnyHook::OnSessionStart(Box::new(CallTrackingHook {
            called: Arc::new(AtomicBool::new(false)),
        })));
        assert!(runner.has_hooks());
    }

    struct PostTurnTrackingHook {
        called: Arc<AtomicBool>,
    }
    #[async_trait]
    impl InspectHook<String> for PostTurnTrackingHook {
        async fn run(&self, _ctx: &mut HookContext, _data: &String) {
            self.called.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_dispatch_post_turn() {
        let called = Arc::new(AtomicBool::new(false));
        let mut runner = HookRunner::new();
        runner.register_hook(AnyHook::PostTurn(Box::new(PostTurnTrackingHook {
            called: called.clone(),
        })));

        let mut turn_ctx = TurnContext::new(&runner.session_context.read().unwrap());
        runner.dispatch_post_turn(&mut turn_ctx, "response").await;
        assert!(called.load(Ordering::SeqCst));
    }

    struct PostToolCallTrackingHook {
        called: Arc<AtomicBool>,
    }
    #[async_trait]
    impl InspectHook<ToolResult> for PostToolCallTrackingHook {
        async fn run(&self, _ctx: &mut HookContext, _data: &ToolResult) {
            self.called.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_dispatch_post_tool_call() {
        let called = Arc::new(AtomicBool::new(false));
        let mut runner = HookRunner::new();
        runner.register_hook(AnyHook::PostToolCall(Box::new(PostToolCallTrackingHook {
            called: called.clone(),
        })));

        let turn_ctx = TurnContext::new(&runner.session_context.read().unwrap());
        let mut op_ctx = OperationContext::new(&turn_ctx);
        let result = ToolResult {
            name: ToolName::Custom("t".to_string()),
            id: None,
            result: None,
            error: None,
        };
        runner.dispatch_post_tool_call(&mut op_ctx, &result).await;
        assert!(called.load(Ordering::SeqCst));
    }

    struct DenyDecideHook;
    #[async_trait]
    impl DecideHook<ToolCall> for DenyDecideHook {
        async fn run(&self, _ctx: &mut HookContext, _data: &ToolCall) -> HookResult {
            HookResult::denied("Denied tool")
        }
    }

    #[tokio::test]
    async fn test_dispatch_pre_tool_call_deny() {
        let mut runner = HookRunner::new();
        runner.register_hook(AnyHook::PreToolCallDecide(Box::new(DenyDecideHook)));
        let turn_ctx = TurnContext::new(&runner.session_context.read().unwrap());
        let tool_call = ToolCall {
            name: ToolName::Custom("t".to_string()),
            args: Default::default(),
            id: None,
            canonical_path: None,
        };

        let (res, _) = runner.dispatch_pre_tool_call(&turn_ctx, &tool_call).await;
        assert!(!res.allow);
        assert_eq!(res.message, "Denied tool");
    }

    struct FallThroughErrorHook;
    #[async_trait]
    impl TransformHook<Box<dyn std::error::Error + Send + Sync>, Option<serde_json::Value>>
        for FallThroughErrorHook
    {
        async fn run(
            &self,
            _ctx: &mut HookContext,
            _data: &Box<dyn std::error::Error + Send + Sync>,
        ) -> Option<serde_json::Value> {
            None
        }
    }

    #[tokio::test]
    async fn test_dispatch_on_tool_error_fall_through() {
        let mut runner = HookRunner::new();
        runner.register_hook(AnyHook::OnToolError(Box::new(FallThroughErrorHook)));
        let turn_ctx = TurnContext::new(&runner.session_context.read().unwrap());
        let mut op_ctx = OperationContext::new(&turn_ctx);

        let error: Box<dyn std::error::Error + Send + Sync> =
            Box::new(crate::types::AntigravityConnectionError {
                message: "Error".to_string(),
            });
        let (res, data) = runner.dispatch_on_tool_error(&mut op_ctx, error).await;

        assert!(!res.allow);
        assert!(data.is_none());
    }

    #[tokio::test]
    async fn test_dispatch_question_delegates_to_interaction() {
        let mut runner = HookRunner::new();
        runner.register_hook(AnyHook::OnInteraction(Box::new(InteractionHook)));
        let turn_ctx = TurnContext::new(&runner.session_context.read().unwrap());
        let spec = AskQuestionInteractionSpec {
            questions: vec![crate::types::AskQuestionEntry {
                question: "magic_question".to_string(),
                options: vec![],
                is_multi_select: false,
            }],
        };
        let (res, answer, _) = runner.dispatch_question(&turn_ctx, &spec).await;
        assert!(res.allow);
        assert_eq!(
            answer.unwrap().responses[0].freeform_response,
            "magic_answer"
        );
    }

    // Testing 25 items would just be permutations. The above captures the core behaviors
    // of all 9 hook dispatching logic.
}
