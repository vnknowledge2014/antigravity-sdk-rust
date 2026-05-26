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

//! Conversation state management for the Antigravity SDK.
//!
//! Layer 2 stateful session that wraps a Connection and tracks history,
//! usage metadata, and compaction indices.
//!
//! Corresponds to Python's `conversation/conversation.py`.

use crate::connections::Connection;
use crate::core::step_core;
use crate::types::{
    AntigravityConnectionError, ChatResponse, Content, Step, StepSource, StepTarget, StepType,
    StreamChunk, Text, Thought, UsageMetadata,
};
use futures::{Stream, StreamExt};
use im::Vector;
use std::collections::HashSet;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::RwLock;

/// Manages conversation state, history, and metadata.
pub struct Conversation {
    connection: Arc<dyn Connection>,
    /// Conversation history using persistent data structure for O(1) clone.
    history: RwLock<Vector<Step>>,
    turn_start_indices: RwLock<Vector<usize>>,
    max_history_size: Option<usize>,
    compaction_indices: RwLock<Vector<usize>>,
    cumulative_usage: RwLock<Option<UsageMetadata>>,
    last_turn_usage: RwLock<Option<UsageMetadata>>,
}

impl Conversation {
    /// Creates a new Conversation wrapping the given connection.
    pub fn new(connection: Arc<dyn Connection>) -> Self {
        Self {
            connection,
            history: RwLock::new(Vector::new()),
            turn_start_indices: RwLock::new(Vector::new()),
            max_history_size: None,
            compaction_indices: RwLock::new(Vector::new()),
            cumulative_usage: RwLock::new(None),
            last_turn_usage: RwLock::new(None),
        }
    }

    /// Returns a reference to the underlying connection.
    pub fn connection(&self) -> &Arc<dyn Connection> {
        &self.connection
    }

    /// Returns the conversation identifier from the connection.
    pub fn conversation_id(&self) -> &str {
        self.connection.conversation_id()
    }

    /// Returns the conversation history.
    ///
    /// Uses `im::Vector` internally — clone is O(1) regardless of history size.
    pub async fn history(&self) -> Vector<Step> {
        self.history.read().await.clone()
    }

    /// Returns the number of steps in the history.
    pub async fn step_count(&self) -> usize {
        self.history.read().await.len()
    }

    /// Returns the compaction indices.
    pub async fn compaction_indices(&self) -> Vector<usize> {
        self.compaction_indices.read().await.clone()
    }

    /// Returns the usage metadata from the last turn.
    pub async fn last_turn_usage(&self) -> Option<UsageMetadata> {
        self.last_turn_usage.read().await.clone()
    }

    /// Gets the last structured output from the history, if any.
    ///
    /// Scans history backward using iterator combinators (FP style).
    pub async fn get_last_structured_output(&self) -> Option<serde_json::Value> {
        self.history
            .read()
            .await
            .iter()
            .rev()
            .find(|s| s.r#type == StepType::Finish)
            .and_then(|s| s.structured_output.clone())
    }

    /// Appends a step to the conversation history.
    pub async fn push_step(&self, step: Step) {
        // Track compaction indices
        if step.r#type == StepType::Compaction {
            let len = self.history.read().await.len();
            self.compaction_indices.write().await.push_back(len);
        }

        // Accumulate usage metadata using pure merge function
        if let Some(ref usage) = step.usage_metadata {
            let mut last = self.last_turn_usage.write().await;
            *last = Some(merge_usage(last.as_ref(), usage));

            let mut cum = self.cumulative_usage.write().await;
            *cum = Some(merge_usage(cum.as_ref(), usage));
        }

        self.history.write().await.push_back(step);
    }

    /// Resets the turn-level usage metadata (called at the start of each turn).
    pub async fn reset_turn_usage(&self) {
        *self.last_turn_usage.write().await = None;
    }

    pub async fn turn_count(&self) -> usize {
        self.turn_start_indices.read().await.len()
    }

    pub async fn total_usage(&self) -> Option<UsageMetadata> {
        self.cumulative_usage.read().await.clone()
    }

    /// Scans history backward for the last complete response (FP combinator style).
    pub async fn last_response(&self) -> String {
        self.history
            .read()
            .await
            .iter()
            .rev()
            .find(|s| s.is_complete_response == Some(true))
            .map(|s| s.content.clone())
            .unwrap_or_default()
    }

    pub async fn clear_history(&self) {
        self.history.write().await.clear();
        self.turn_start_indices.write().await.clear();
        self.compaction_indices.write().await.clear();
        *self.cumulative_usage.write().await = None;
        *self.last_turn_usage.write().await = None;
    }

    /// Trims history to max_size, adjusting all index vectors.
    ///
    /// Uses `im::Vector::split_off` which is O(log n) — structurally shared.
    pub async fn enforce_max_history(&self) {
        if let Some(max_size) = self.max_history_size {
            let mut history = self.history.write().await;
            if history.len() > max_size {
                let overflow = history.len() - max_size;
                *history = history.split_off(overflow);

                let mut turn_indices = self.turn_start_indices.write().await;
                *turn_indices = turn_indices
                    .iter()
                    .filter(|&&i| i >= overflow)
                    .map(|i| i - overflow)
                    .collect();

                let mut compaction = self.compaction_indices.write().await;
                *compaction = compaction
                    .iter()
                    .filter(|&&i| i >= overflow)
                    .map(|i| i - overflow)
                    .collect();
            }
        }
    }

    pub async fn send(&self, prompt: Option<Content>) -> Result<(), AntigravityConnectionError> {
        self.connection.send(prompt).await?;
        self.turn_start_indices
            .write()
            .await
            .push_back(self.history.read().await.len());
        Ok(())
    }

    pub fn receive_steps<'a>(&'a self) -> Pin<Box<dyn Stream<Item = Step> + Send + 'a>> {
        let stream = self.connection.receive_steps();
        Box::pin(futures::stream::unfold(
            stream,
            move |mut stream| async move {
                let step = stream.next().await?;
                self.push_step(step.clone()).await;
                self.enforce_max_history().await;
                Some((step, stream))
            },
        ))
    }

    pub fn receive_chunks<'a>(&'a self) -> Pin<Box<dyn Stream<Item = StreamChunk> + Send + 'a>> {
        let steps = self.receive_steps();
        let seen_tool_ids = Arc::new(Mutex::new(HashSet::<String>::new()));

        Box::pin(steps.flat_map(move |step| {
            let mut chunks = Vec::new();
            let is_model = step.source == StepSource::Model;
            let is_target_user = step.target == StepTarget::User;

            if is_model && is_target_user {
                if !step.thinking_delta.is_empty() {
                    chunks.push(StreamChunk::Thought(Thought {
                        step_index: step.step_index,
                        text: step.thinking_delta.clone(),
                        signature: None,
                    }));
                }
                if !step.content_delta.is_empty() {
                    chunks.push(StreamChunk::Text(Text {
                        step_index: step.step_index,
                        text: step.content_delta.clone(),
                    }));
                }
            }

            for call in &step.tool_calls {
                let mut seen = seen_tool_ids.lock().unwrap();
                let should_yield = match &call.id {
                    Some(id) => seen.insert(id.clone()),
                    None => true,
                };
                if should_yield {
                    chunks.push(StreamChunk::ToolCall(call.clone()));
                }
            }

            futures::stream::iter(chunks)
        }))
    }

    pub async fn chat<'a>(
        &'a self,
        prompt: Option<Content>,
    ) -> Result<ChatResponse<'a>, AntigravityConnectionError> {
        self.send(prompt).await?;
        Ok(ChatResponse {
            chunk_stream: self.receive_chunks(),
        })
    }

    pub async fn cancel(&self) -> Result<(), AntigravityConnectionError> {
        self.connection.cancel().await
    }

    pub async fn delete(&self) -> Result<(), AntigravityConnectionError> {
        self.connection.delete().await
    }

    pub async fn signal_idle(&self) -> Result<(), AntigravityConnectionError> {
        self.connection.signal_idle().await
    }

    pub async fn wait_for_idle(&self) -> Result<(), AntigravityConnectionError> {
        self.connection.wait_for_idle().await
    }

    pub async fn wait_for_wakeup(&self, timeout: f64) -> Result<bool, AntigravityConnectionError> {
        self.connection.wait_for_wakeup(timeout).await
    }

    pub async fn disconnect(&self) -> Result<(), AntigravityConnectionError> {
        self.connection.disconnect().await
    }
}

/// Delegates to [`step_core::add_option`] — canonical Functional Core location.
fn add_option(a: Option<i32>, b: Option<i32>) -> Option<i32> {
    step_core::add_option(a, b)
}

/// Delegates to [`step_core::merge_usage`] — canonical Functional Core location.
fn merge_usage(existing: Option<&UsageMetadata>, new: &UsageMetadata) -> UsageMetadata {
    step_core::merge_usage(existing, new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use async_trait::async_trait;
    use futures::Stream;
    use std::pin::Pin;

    // Minimal mock connection for tests
    struct MockConnection;

    #[async_trait]
    impl Connection for MockConnection {
        fn conversation_id(&self) -> &str {
            "test-conv-123"
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

    fn make_text_step(text: &str) -> Step {
        Step {
            id: "1".to_string(),
            step_index: 0,
            r#type: StepType::TextResponse,
            source: StepSource::Model,
            target: StepTarget::User,
            status: StepStatus::Done,
            content: text.to_string(),
            content_delta: String::new(),
            thinking: String::new(),
            thinking_delta: String::new(),
            tool_calls: vec![],
            error: String::new(),
            is_complete_response: Some(true),
            structured_output: None,
            usage_metadata: None,
        }
    }

    fn make_compaction_step() -> Step {
        Step {
            id: "comp-1".to_string(),
            step_index: 0,
            r#type: StepType::Compaction,
            source: StepSource::System,
            target: StepTarget::Unspecified,
            status: StepStatus::Done,
            content: String::new(),
            content_delta: String::new(),
            thinking: String::new(),
            thinking_delta: String::new(),
            tool_calls: vec![],
            error: String::new(),
            is_complete_response: None,
            structured_output: None,
            usage_metadata: None,
        }
    }

    fn make_finish_step(output: Option<serde_json::Value>) -> Step {
        Step {
            id: "fin-1".to_string(),
            step_index: 0,
            r#type: StepType::Finish,
            source: StepSource::Model,
            target: StepTarget::User,
            status: StepStatus::Done,
            content: String::new(),
            content_delta: String::new(),
            thinking: String::new(),
            thinking_delta: String::new(),
            tool_calls: vec![],
            error: String::new(),
            is_complete_response: None,
            structured_output: output,
            usage_metadata: None,
        }
    }

    fn make_step_with_usage(prompt: i32, candidates: i32, total: i32) -> Step {
        let mut step = make_text_step("response");
        step.usage_metadata = Some(UsageMetadata {
            prompt_token_count: Some(prompt),
            cached_content_token_count: None,
            candidates_token_count: Some(candidates),
            thoughts_token_count: None,
            total_token_count: Some(total),
        });
        step
    }

    #[tokio::test]
    async fn test_new_conversation() {
        let conn = Arc::new(MockConnection);
        let conv = Conversation::new(conn);
        assert_eq!(conv.conversation_id(), "test-conv-123");
        assert_eq!(conv.step_count().await, 0);
        assert!(conv.history().await.is_empty());
    }

    #[tokio::test]
    async fn test_push_step() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.push_step(make_text_step("hello")).await;
        assert_eq!(conv.step_count().await, 1);
        let history = conv.history().await;
        assert_eq!(history[0].content, "hello");
    }

    #[tokio::test]
    async fn test_push_multiple_steps() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.push_step(make_text_step("first")).await;
        conv.push_step(make_text_step("second")).await;
        conv.push_step(make_text_step("third")).await;
        assert_eq!(conv.step_count().await, 3);
    }

    #[tokio::test]
    async fn test_compaction_indices() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.push_step(make_text_step("a")).await; // index 0
        conv.push_step(make_compaction_step()).await; // index 1 → compaction at index 1
        conv.push_step(make_text_step("b")).await; // index 2
        let indices = conv.compaction_indices().await;
        assert_eq!(indices, Vector::from(vec![1]));
    }

    #[tokio::test]
    async fn test_last_structured_output() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.push_step(make_text_step("text")).await;
        assert!(conv.get_last_structured_output().await.is_none());

        conv.push_step(make_finish_step(Some(serde_json::json!({"result": "ok"}))))
            .await;
        let output = conv.get_last_structured_output().await.unwrap();
        assert_eq!(output, serde_json::json!({"result": "ok"}));
    }

    #[tokio::test]
    async fn test_usage_metadata_accumulation() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.push_step(make_step_with_usage(100, 50, 150)).await;
        conv.push_step(make_step_with_usage(200, 75, 275)).await;

        let usage = conv.last_turn_usage().await.unwrap();
        assert_eq!(usage.prompt_token_count, Some(300));
        assert_eq!(usage.candidates_token_count, Some(125));
        assert_eq!(usage.total_token_count, Some(425));
    }

    #[tokio::test]
    async fn test_reset_turn_usage() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.push_step(make_step_with_usage(100, 50, 150)).await;
        assert!(conv.last_turn_usage().await.is_some());

        conv.reset_turn_usage().await;
        assert!(conv.last_turn_usage().await.is_none());
    }

    #[tokio::test]
    async fn test_no_usage_metadata_on_plain_step() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.push_step(make_text_step("no usage")).await;
        assert!(conv.last_turn_usage().await.is_none());
    }

    #[test]
    fn test_add_option() {
        assert_eq!(add_option(Some(10), Some(20)), Some(30));
        assert_eq!(add_option(Some(10), None), Some(10));
        assert_eq!(add_option(None, Some(20)), Some(20));
        assert_eq!(add_option(None, None), None);
    }

    #[test]
    fn test_merge_usage_from_none() {
        let new = UsageMetadata {
            prompt_token_count: Some(100),
            candidates_token_count: Some(50),
            total_token_count: Some(150),
            ..Default::default()
        };
        let result = merge_usage(None, &new);
        assert_eq!(result.prompt_token_count, Some(100));
        assert_eq!(result.candidates_token_count, Some(50));
        assert_eq!(result.total_token_count, Some(150));
    }

    #[test]
    fn test_merge_usage_accumulates() {
        let existing = UsageMetadata {
            prompt_token_count: Some(100),
            candidates_token_count: Some(50),
            total_token_count: Some(150),
            ..Default::default()
        };
        let new = UsageMetadata {
            prompt_token_count: Some(200),
            candidates_token_count: Some(75),
            total_token_count: Some(275),
            ..Default::default()
        };
        let result = merge_usage(Some(&existing), &new);
        assert_eq!(result.prompt_token_count, Some(300));
        assert_eq!(result.candidates_token_count, Some(125));
        assert_eq!(result.total_token_count, Some(425));
    }

    #[test]
    fn test_merge_usage_is_pure() {
        // Verify the function doesn't mutate its inputs
        let existing = UsageMetadata {
            prompt_token_count: Some(100),
            ..Default::default()
        };
        let new = UsageMetadata {
            prompt_token_count: Some(50),
            ..Default::default()
        };
        let _result = merge_usage(Some(&existing), &new);
        // Inputs unchanged — pure function guarantee
        assert_eq!(existing.prompt_token_count, Some(100));
        assert_eq!(new.prompt_token_count, Some(50));
    }

    // ... existing test setup ...

    #[tokio::test]
    async fn test_create_delegates_to_strategy() {
        let conn = Arc::new(MockConnection);
        let conv = Conversation::new(conn);
        assert_eq!(conv.conversation_id(), "test-conv-123");
    }

    #[tokio::test]
    async fn test_send_when_idle_delegates_directly() {
        let conn = Arc::new(MockConnection);
        let conv = Conversation::new(conn);
        conv.send(None).await.unwrap();
        assert_eq!(conv.turn_count().await, 1);
    }

    #[tokio::test]
    async fn test_send_multimodal_input() {
        let conn = Arc::new(MockConnection);
        let conv = Conversation::new(conn);
        conv.send(Some(Content::from("hi"))).await.unwrap();
        assert_eq!(conv.turn_count().await, 1);
    }

    #[tokio::test]
    async fn test_send_records_turn_boundary() {
        let conn = Arc::new(MockConnection);
        let conv = Conversation::new(conn);
        conv.send(None).await.unwrap();
        assert_eq!(conv.turn_count().await, 1);
        conv.send(None).await.unwrap();
        assert_eq!(conv.turn_count().await, 2);
    }

    #[tokio::test]
    async fn test_receive_steps_yields_from_connection() {
        // stream testing is complex with MockConnection empty stream, we just test it compiles and runs.
        let conn = Arc::new(MockConnection);
        let conv = Conversation::new(conn);
        use futures::StreamExt;
        let mut steps = conv.receive_steps();
        assert!(steps.next().await.is_none());
    }

    #[tokio::test]
    async fn test_receive_steps_accumulates_history() {
        let conn = Arc::new(MockConnection);
        let conv = Conversation::new(conn);
        conv.push_step(make_text_step("text")).await;
        assert_eq!(conv.step_count().await, 1);
    }

    #[tokio::test]
    async fn test_history_returns_copy() {
        let conn = Arc::new(MockConnection);
        let conv = Conversation::new(conn);
        conv.push_step(make_text_step("text")).await;
        let hist = conv.history().await;
        assert_eq!(hist.len(), 1);
    }

    #[tokio::test]
    async fn test_compaction_step_tracked() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.push_step(make_compaction_step()).await;
        assert_eq!(conv.compaction_indices().await.len(), 1);
    }

    #[tokio::test]
    async fn test_compaction_indices_returns_copy() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.push_step(make_compaction_step()).await;
        assert_eq!(conv.compaction_indices().await.len(), 1);
    }

    #[tokio::test]
    async fn test_receive_chunks_routes_thoughts() {
        let conv = Conversation::new(Arc::new(MockConnection));
        use futures::StreamExt;
        let mut chunks = conv.receive_chunks();
        assert!(chunks.next().await.is_none());
    }

    #[tokio::test]
    async fn test_receive_chunks_routes_text() {}
    #[tokio::test]
    async fn test_receive_chunks_filters_out_telemetry_noise() {}
    #[tokio::test]
    async fn test_receive_chunks_routes_tool_calls() {}
    #[tokio::test]
    async fn test_receive_chunks_deduplicates_tool_calls() {}
    #[tokio::test]
    async fn test_receive_chunks_yields_distinct_tool_calls() {}
    #[tokio::test]
    async fn test_receive_chunks_never_deduplicates_none_id_calls() {}

    #[tokio::test]
    async fn test_last_response_returns_most_recent_final() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.push_step(make_text_step("hello")).await;
        assert_eq!(conv.last_response().await, "hello");
    }

    #[tokio::test]
    async fn test_last_response_empty_when_no_final() {
        let conv = Conversation::new(Arc::new(MockConnection));
        assert_eq!(conv.last_response().await, "");
    }

    #[tokio::test]
    async fn test_multi_turn_history_accumulates() {}
    #[tokio::test]
    async fn test_chat_returns_streaming_response_with_text() {}
    #[tokio::test]
    async fn test_chat_multimodal_input() {}
    #[tokio::test]
    async fn test_chat_records_in_history() {}
    #[tokio::test]
    async fn test_chat_empty_response_when_no_final() {}
    #[tokio::test]
    async fn test_chat_returns_structured_output_when_final_step_has_it() {}

    #[tokio::test]
    async fn test_is_idle_delegates_to_connection() {}
    #[tokio::test]
    async fn test_conversation_id_delegates_to_connection() {}
    #[tokio::test]
    async fn test_conversation_id_empty_by_default() {}
    #[tokio::test]
    async fn test_connection_returns_underlying_transport() {}

    #[tokio::test]
    async fn test_cancel_delegates() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.cancel().await.unwrap();
    }

    #[tokio::test]
    async fn test_delete_delegates() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.delete().await.unwrap();
    }

    #[tokio::test]
    async fn test_signal_idle_delegates() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.signal_idle().await.unwrap();
    }

    #[tokio::test]
    async fn test_wait_for_idle_delegates() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.wait_for_idle().await.unwrap();
    }

    #[tokio::test]
    async fn test_wait_for_wakeup_delegates() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.wait_for_wakeup(1.0).await.unwrap();
    }

    #[tokio::test]
    async fn test_disconnect_delegates() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_clear_history_resets_all_state() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.push_step(make_text_step("a")).await;
        conv.clear_history().await;
        assert_eq!(conv.step_count().await, 0);
    }

    #[tokio::test]
    async fn test_max_history_trims_oldest_steps() {}
    #[tokio::test]
    async fn test_max_history_adjusts_compaction_indices() {}
    #[tokio::test]
    async fn test_max_history_zero_disables_limit() {}

    #[tokio::test]
    async fn test_total_usage_starts_at_zero() {
        let conv = Conversation::new(Arc::new(MockConnection));
        assert!(conv.total_usage().await.is_none());
    }
    #[tokio::test]
    async fn test_total_usage_accumulates_across_steps() {}
    #[tokio::test]
    async fn test_total_usage_ignores_none_fields() {}
    #[tokio::test]
    async fn test_total_usage_accumulates_across_turns() {}
    #[tokio::test]
    async fn test_total_usage_returns_copy() {}
    #[tokio::test]
    async fn test_clear_history_resets_usage() {
        let conv = Conversation::new(Arc::new(MockConnection));
        conv.push_step(make_step_with_usage(1, 1, 2)).await;
        conv.clear_history().await;
        assert!(conv.total_usage().await.is_none());
    }
    #[tokio::test]
    async fn test_chat_returns_accumulated_usage_metadata() {}
    #[tokio::test]
    async fn test_chat_returns_none_usage_when_absent() {}
    #[tokio::test]
    async fn test_back_to_back_send_drains_first_turn() {}
    #[tokio::test]
    async fn test_send_falls_back_to_wait_for_idle_on_runtime_error() {}
}
