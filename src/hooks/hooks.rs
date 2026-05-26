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

//! Hook trait definitions for the Antigravity SDK.
//!
//! Defines the core hook abstractions: `InspectHook`, `DecideHook`,
//! `TransformHook`, plus context types and concrete hook type aliases.
//!
//! Corresponds to Python's `hooks/hooks.py`.

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;

use crate::types::{
    AskQuestionInteractionSpec, Content, HookResult, QuestionHookResult, Step, ToolCall, ToolResult,
};

// =============================================================================
// Contexts — hierarchical state containers
// =============================================================================

/// Base context for hooks to share state via a key-value store.
#[derive(Debug, Default)]

pub struct HookContext {
    store: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl HookContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get<T: 'static + Send + Sync>(&self, key: &str) -> Option<&T> {
        self.store.get(key)?.downcast_ref()
    }

    pub fn set<T: 'static + Send + Sync>(&mut self, key: String, value: T) {
        self.store.insert(key, Box::new(value));
    }
}

/// Context scoped to an entire session.
#[derive(Debug, Default)]
pub struct SessionContext {
    pub ctx: HookContext,
}

/// Context scoped to a single turn.
#[derive(Debug)]

pub struct TurnContext {
    pub ctx: HookContext,
    pub turn_id: String,
    pub step_count: u32,
    pub metadata: HashMap<String, String>,
}

impl Default for TurnContext {
    fn default() -> Self {
        Self {
            ctx: HookContext::new(),
            turn_id: String::new(),
            step_count: 0,
            metadata: HashMap::new(),
        }
    }
}

impl TurnContext {
    pub fn new(_session: &SessionContext) -> Self {
        Self {
            ctx: HookContext::new(),
            turn_id: String::new(),
            step_count: 0,
            metadata: HashMap::new(),
        }
    }
}

/// Context scoped to a specific operation (e.g. tool call).
#[derive(Debug)]
pub struct OperationContext {
    pub ctx: HookContext,
}

impl OperationContext {
    pub fn new(_turn: &TurnContext) -> Self {
        Self {
            ctx: HookContext::new(),
        }
    }
}

// =============================================================================
// Hook traits — the core abstractions
// =============================================================================

/// Read-only, non-blocking hook for observability.
#[async_trait]
pub trait InspectHook<T: Send + Sync>: Send + Sync {
    async fn run(&self, context: &mut HookContext, data: &T);
}

/// Read-only, blocking hook for policy decisions.
#[async_trait]
pub trait DecideHook<T: Send + Sync>: Send + Sync {
    async fn run(&self, context: &mut HookContext, data: &T) -> HookResult;
}

/// Modifying, blocking hook for data transformation.
#[async_trait]
pub trait TransformHook<T: Send + Sync, R: Send + Sync>: Send + Sync {
    async fn run(&self, context: &mut HookContext, data: &T) -> R;
}

// =============================================================================
// Concrete hook type aliases using trait objects
// =============================================================================

pub type OnSessionStartHook = dyn InspectHook<()> + Send + Sync;
pub type OnSessionEndHook = dyn InspectHook<()> + Send + Sync;
pub type PreTurnHook = dyn DecideHook<Content> + Send + Sync;
pub type PostTurnHook = dyn InspectHook<String> + Send + Sync;
pub type PreToolCallDecideHook = dyn DecideHook<ToolCall> + Send + Sync;
pub type PostToolCallHook = dyn InspectHook<ToolResult> + Send + Sync;
pub type OnToolErrorHook = dyn TransformHook<Box<dyn std::error::Error + Send + Sync>, Option<serde_json::Value>>
    + Send
    + Sync;
pub type OnInteractionHook =
    dyn TransformHook<AskQuestionInteractionSpec, Option<QuestionHookResult>> + Send + Sync;
pub type OnCompactionHook = dyn InspectHook<Step> + Send + Sync;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_context_get_set() {
        let mut ctx = HookContext::new();
        ctx.set("key".to_string(), 42i32);
        assert_eq!(ctx.get::<i32>("key"), Some(&42));
    }

    #[test]
    fn test_hook_context_get_missing() {
        let ctx = HookContext::new();
        assert_eq!(ctx.get::<i32>("missing"), None);
    }

    #[test]
    fn test_hook_context_type_mismatch() {
        let mut ctx = HookContext::new();
        ctx.set("key".to_string(), "hello".to_string());
        assert_eq!(ctx.get::<i32>("key"), None); // Wrong type
    }

    #[test]
    fn test_hook_context_overwrite() {
        let mut ctx = HookContext::new();
        ctx.set("key".to_string(), 1i32);
        ctx.set("key".to_string(), 2i32);
        assert_eq!(ctx.get::<i32>("key"), Some(&2));
    }

    #[test]
    fn test_session_context_default() {
        let session = SessionContext::default();
        assert_eq!(session.ctx.get::<i32>("anything"), None);
    }

    #[test]
    fn test_turn_context_has_own_store() {
        let session = SessionContext::default();
        let mut turn = TurnContext::new(&session);
        turn.ctx.set("turn_key".to_string(), 42i32);
        assert_eq!(turn.ctx.get::<i32>("turn_key"), Some(&42));
    }

    #[test]
    fn test_operation_context_has_own_store() {
        let session = SessionContext::default();
        let turn = TurnContext::new(&session);
        let mut op = OperationContext::new(&turn);
        op.ctx.set("op_key".to_string(), "value".to_string());
        assert_eq!(op.ctx.get::<String>("op_key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_contexts_are_independent() {
        let session = SessionContext::default();
        let mut turn1 = TurnContext::new(&session);
        let mut turn2 = TurnContext::new(&session);
        turn1.ctx.set("k".to_string(), 1i32);
        turn2.ctx.set("k".to_string(), 2i32);
        assert_eq!(turn1.ctx.get::<i32>("k"), Some(&1));
        assert_eq!(turn2.ctx.get::<i32>("k"), Some(&2));
    }
}
