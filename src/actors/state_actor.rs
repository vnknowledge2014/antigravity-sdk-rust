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

//! StateActor — central state owner for a connection session.
//!
//! Replaces the 8 `Arc<Mutex<...>>` fields that were previously scattered
//! across `LocalConnection`. All mutable state lives here, accessed
//! exclusively through message passing.
//!
//! # Zero-lock guarantee
//!
//! The actor owns all state fields outright. No `Mutex` or `RwLock` is needed
//! because only the actor's single `tokio::spawn` task accesses them.

use std::collections::{HashMap, HashSet, VecDeque};
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};

use crate::hooks::TurnContext;
use crate::types::Step;

/// Messages the StateActor can receive.
///
/// Each variant represents either a command (fire-and-forget) or a query
/// (carries a `oneshot::Sender` for the reply).
#[derive(Debug)]
pub enum StateMsg {
    // ── Commands (fire-and-forget) ──────────────────────────────────

    /// A new step was received from the WebSocket reader.
    StepReceived(Step),

    /// The connection became idle or active.
    IdleChanged(bool),

    /// Update the cascade (session) ID.
    SetCascadeId(Option<String>),

    /// Track a subagent as active.
    AddSubagent(String),

    /// Remove a subagent from active set.
    RemoveSubagent(String),

    /// Store a subagent response.
    StoreSubagentResponse { id: String, response: String },

    /// Append a stderr line (bounded ring buffer).
    AppendStderr(String),

    /// Set the current turn context.
    SetTurnContext(Option<TurnContext>),

    // ── Queries (with reply channel) ────────────────────────────────

    /// Get the current cascade ID.
    GetCascadeId(oneshot::Sender<Option<String>>),

    /// Check if a turn context is currently set.
    HasTurnContext(oneshot::Sender<bool>),

    /// Take the turn context (moves ownership out of the actor).
    TakeTurnContext(oneshot::Sender<Option<TurnContext>>),

    /// Get active subagent count.
    GetActiveSubagentCount(oneshot::Sender<usize>),

    /// Get stderr lines snapshot.
    GetStderrLines(oneshot::Sender<Vec<String>>),

    /// Get a snapshot of all state for diagnostics.
    GetSnapshot(oneshot::Sender<StateSnapshot>),

    /// Shut down the actor.
    Shutdown,
}

/// Diagnostic snapshot of actor state.
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub cascade_id: Option<String>,
    pub active_subagent_count: usize,
    pub stderr_line_count: usize,
    pub step_count: usize,
    pub has_turn_context: bool,
}

/// The central state actor — owns ALL mutable connection state.
///
/// Fields that were previously `Arc<Mutex<T>>` in `LocalConnection`
/// are now plain owned fields here. Zero locks required.
pub struct StateActor {
    // ── Owned state (was Arc<Mutex<...>>) ───────────────────────────
    turn_context: Option<TurnContext>,
    active_subagent_ids: HashSet<String>,
    subagent_responses: HashMap<String, String>,
    stderr_lines: VecDeque<String>,
    cascade_id: Option<String>,

    // ── Counters ────────────────────────────────────────────────────
    step_count: usize,

    // ── Channels ────────────────────────────────────────────────────
    msg_rx: mpsc::UnboundedReceiver<StateMsg>,
    step_tx: mpsc::UnboundedSender<Step>,

    // ── Config ──────────────────────────────────────────────────────
    max_stderr_lines: usize,
}

impl StateActor {
    /// Creates a new StateActor.
    ///
    /// - `msg_rx`: channel to receive commands/queries
    /// - `step_tx`: channel to forward steps to the Conversation layer
    pub fn new(
        msg_rx: mpsc::UnboundedReceiver<StateMsg>,
        step_tx: mpsc::UnboundedSender<Step>,
    ) -> Self {
        Self {
            turn_context: None,
            active_subagent_ids: HashSet::new(),
            subagent_responses: HashMap::new(),
            stderr_lines: VecDeque::new(),
            cascade_id: None,
            step_count: 0,
            msg_rx,
            step_tx,
            max_stderr_lines: 1000,
        }
    }

    /// Runs the actor event loop.
    ///
    /// Processes messages sequentially. Returns when the channel is closed
    /// or a `Shutdown` message is received.
    pub async fn run(mut self) {
        info!("StateActor started");
        while let Some(msg) = self.msg_rx.recv().await {
            match msg {
                // ── Commands ────────────────────────────────────────
                StateMsg::StepReceived(step) => {
                    self.handle_step(step);
                }
                StateMsg::IdleChanged(idle) => {
                    if idle {
                        info!("StateActor: connection became idle");
                    }
                }
                StateMsg::SetCascadeId(id) => {
                    self.cascade_id = id;
                }
                StateMsg::AddSubagent(id) => {
                    self.active_subagent_ids.insert(id);
                }
                StateMsg::RemoveSubagent(id) => {
                    self.active_subagent_ids.remove(&id);
                }
                StateMsg::StoreSubagentResponse { id, response } => {
                    self.subagent_responses.insert(id, response);
                }
                StateMsg::AppendStderr(line) => {
                    self.append_stderr(line);
                }
                StateMsg::SetTurnContext(ctx) => {
                    self.turn_context = ctx;
                }

                // ── Queries ─────────────────────────────────────────
                StateMsg::GetCascadeId(reply) => {
                    let _ = reply.send(self.cascade_id.clone());
                }
                StateMsg::HasTurnContext(reply) => {
                    let _ = reply.send(self.turn_context.is_some());
                }
                StateMsg::TakeTurnContext(reply) => {
                    let _ = reply.send(self.turn_context.take());
                }
                StateMsg::GetActiveSubagentCount(reply) => {
                    let _ = reply.send(self.active_subagent_ids.len());
                }
                StateMsg::GetStderrLines(reply) => {
                    let lines: Vec<String> = self.stderr_lines.iter().cloned().collect();
                    let _ = reply.send(lines);
                }
                StateMsg::GetSnapshot(reply) => {
                    let _ = reply.send(StateSnapshot {
                        cascade_id: self.cascade_id.clone(),
                        active_subagent_count: self.active_subagent_ids.len(),
                        stderr_line_count: self.stderr_lines.len(),
                        step_count: self.step_count,
                        has_turn_context: self.turn_context.is_some(),
                    });
                }

                // ── Lifecycle ───────────────────────────────────────
                StateMsg::Shutdown => {
                    info!("StateActor shutting down");
                    break;
                }
            }
        }
        info!("StateActor stopped (processed {} steps)", self.step_count);
    }

    // ── Private helpers ─────────────────────────────────────────────

    /// Forwards a step to the Conversation layer via channel.
    fn handle_step(&mut self, step: Step) {
        self.step_count += 1;
        if self.step_tx.send(step).is_err() {
            warn!("StateActor: step_tx closed, step dropped");
        }
    }

    /// Appends a stderr line, enforcing the ring buffer limit.
    fn append_stderr(&mut self, line: String) {
        if self.stderr_lines.len() >= self.max_stderr_lines {
            self.stderr_lines.pop_front();
        }
        self.stderr_lines.push_back(line);
    }
}

// ── Convenience helper for sending queries ──────────────────────────

/// Send a query to the StateActor and await the response.
///
/// This is a helper to reduce boilerplate when querying the actor
/// from the imperative shell (LocalConnection façade).
pub async fn query<T>(
    tx: &mpsc::UnboundedSender<StateMsg>,
    make_msg: impl FnOnce(oneshot::Sender<T>) -> StateMsg,
) -> Option<T> {
    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send(make_msg(reply_tx)).ok()?;
    reply_rx.await.ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_step() -> Step {
        Step {
            r#type: crate::types::StepType::TextResponse,
            source: crate::types::StepSource::Model,
            target: crate::types::StepTarget::User,
            content: "hello".into(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_state_actor_step_forwarding() {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (step_tx, mut step_rx) = mpsc::unbounded_channel();

        let actor = StateActor::new(msg_rx, step_tx);
        let handle = tokio::spawn(actor.run());

        msg_tx.send(StateMsg::StepReceived(make_test_step())).unwrap();
        msg_tx.send(StateMsg::Shutdown).unwrap();

        handle.await.unwrap();

        let step = step_rx.recv().await.unwrap();
        assert_eq!(step.content, "hello");
    }

    #[tokio::test]
    async fn test_state_actor_cascade_id() {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (step_tx, _) = mpsc::unbounded_channel();

        let actor = StateActor::new(msg_rx, step_tx);
        let handle = tokio::spawn(actor.run());

        // Initially None
        let id = query(&msg_tx, StateMsg::GetCascadeId).await.unwrap();
        assert_eq!(id, None);

        // Set cascade ID
        msg_tx.send(StateMsg::SetCascadeId(Some("session-42".into()))).unwrap();
        let id = query(&msg_tx, StateMsg::GetCascadeId).await.unwrap();
        assert_eq!(id, Some("session-42".into()));

        msg_tx.send(StateMsg::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_state_actor_subagents() {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (step_tx, _) = mpsc::unbounded_channel();

        let actor = StateActor::new(msg_rx, step_tx);
        let handle = tokio::spawn(actor.run());

        // Add subagents
        msg_tx.send(StateMsg::AddSubagent("sub-1".into())).unwrap();
        msg_tx.send(StateMsg::AddSubagent("sub-2".into())).unwrap();

        let count = query(&msg_tx, StateMsg::GetActiveSubagentCount).await.unwrap();
        assert_eq!(count, 2);

        // Remove one
        msg_tx.send(StateMsg::RemoveSubagent("sub-1".into())).unwrap();
        let count = query(&msg_tx, StateMsg::GetActiveSubagentCount).await.unwrap();
        assert_eq!(count, 1);

        msg_tx.send(StateMsg::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_state_actor_stderr_ring_buffer() {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (step_tx, _) = mpsc::unbounded_channel();

        let mut actor = StateActor::new(msg_rx, step_tx);
        actor.max_stderr_lines = 3; // Small buffer for test

        let handle = tokio::spawn(actor.run());

        for i in 0..5 {
            msg_tx.send(StateMsg::AppendStderr(format!("line-{i}"))).unwrap();
        }

        let lines = query(&msg_tx, StateMsg::GetStderrLines).await.unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line-2"); // Oldest dropped
        assert_eq!(lines[2], "line-4");

        msg_tx.send(StateMsg::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_state_actor_snapshot() {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (step_tx, _) = mpsc::unbounded_channel();

        let actor = StateActor::new(msg_rx, step_tx);
        let handle = tokio::spawn(actor.run());

        msg_tx.send(StateMsg::SetCascadeId(Some("s1".into()))).unwrap();
        msg_tx.send(StateMsg::AddSubagent("sub-a".into())).unwrap();
        msg_tx.send(StateMsg::StepReceived(make_test_step())).unwrap();
        msg_tx.send(StateMsg::StepReceived(make_test_step())).unwrap();

        let snap = query(&msg_tx, StateMsg::GetSnapshot).await.unwrap();
        assert_eq!(snap.cascade_id, Some("s1".into()));
        assert_eq!(snap.active_subagent_count, 1);
        assert_eq!(snap.step_count, 2);
        assert!(!snap.has_turn_context);

        msg_tx.send(StateMsg::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_state_actor_turn_context() {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (step_tx, _) = mpsc::unbounded_channel();

        let actor = StateActor::new(msg_rx, step_tx);
        let handle = tokio::spawn(actor.run());

        // Initially no context
        let has = query(&msg_tx, StateMsg::HasTurnContext).await.unwrap();
        assert!(!has);

        // Set context
        let tc = TurnContext::default();
        msg_tx.send(StateMsg::SetTurnContext(Some(tc))).unwrap();

        let has = query(&msg_tx, StateMsg::HasTurnContext).await.unwrap();
        assert!(has);

        // Take context (moves out)
        let taken = query(&msg_tx, StateMsg::TakeTurnContext).await.unwrap();
        assert!(taken.is_some());

        // After take, should be None
        let has = query(&msg_tx, StateMsg::HasTurnContext).await.unwrap();
        assert!(!has);

        msg_tx.send(StateMsg::Shutdown).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_query_helper_closed_channel() {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel::<StateMsg>();
        drop(msg_rx); // Close receiver immediately

        let result = query(&msg_tx, StateMsg::GetCascadeId).await;
        assert!(result.is_none()); // Should return None, not panic
    }

    #[tokio::test]
    async fn test_state_actor_channel_close_stops() {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (step_tx, _) = mpsc::unbounded_channel();

        let actor = StateActor::new(msg_rx, step_tx);
        let handle = tokio::spawn(actor.run());

        drop(msg_tx); // Close channel
        handle.await.unwrap(); // Should terminate cleanly
    }
}
