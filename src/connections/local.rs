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

// Local connection implementation for the Google Antigravity SDK.
//
// Implements the concrete `LocalConnection` and `LocalConnectionStrategy`
// that communicate with the Go-based localharness binary via WebSocket.
//
// Corresponds to Python's `local_connection.py`.

use async_trait::async_trait;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::process::Child;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::connections::wire_types::localharness::*;
use crate::connections::{Connection, ConnectionStrategy};
use crate::core::tool_core;
use crate::hooks::{HookRunner, OperationContext, TurnContext};
use crate::tools::ToolRunner;
use crate::types::{self, *};

// Re-export for backward compatibility — examples import from connections::local
pub use crate::connections::config::LocalAgentConfig;

// =============================================================================
// Source/Status maps (matching Python's _SOURCE_MAP and _STATUS_MAP)
// =============================================================================

fn parse_source(s: &str) -> StepSource {
    match s {
        "SOURCE_SYSTEM" => StepSource::System,
        "SOURCE_USER" => StepSource::User,
        "SOURCE_MODEL" => StepSource::Model,
        _ => StepSource::Unknown,
    }
}

fn parse_status(s: &str) -> StepStatus {
    match s {
        "STATE_ACTIVE" => StepStatus::Active,
        "STATE_DONE" => StepStatus::Done,
        "STATE_WAITING_FOR_USER" => StepStatus::WaitingForUser,
        "STATE_ERROR" => StepStatus::Error,
        _ => StepStatus::Unknown,
    }
}

/// Map from BuiltinTools to the proto field name on StepUpdate.
const BUILTIN_TOOL_FIELDS: &[(BuiltinTools, &str)] = &[
    (BuiltinTools::CreateFile, "create_file"),
    (BuiltinTools::EditFile, "edit_file"),
    (BuiltinTools::FindFile, "find_file"),
    (BuiltinTools::ListDir, "list_directory"),
    (BuiltinTools::RunCommand, "run_command"),
    (BuiltinTools::SearchDir, "search_directory"),
    (BuiltinTools::ViewFile, "view_file"),
    (BuiltinTools::StartSubagent, "invoke_subagent"),
    (BuiltinTools::GenerateImage, "generate_image"),
    (BuiltinTools::Finish, "finish"),
];

// =============================================================================
// Helper functions
// =============================================================================

/// Translates Go harness transport representations to clean absolute paths.
pub fn normalize_wire_path(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("file://") {
        url::form_urlencoded::parse(stripped.as_bytes())
            .map(|(k, _)| k.into_owned())
            .next()
            .unwrap_or_else(|| stripped.to_string())
    } else {
        path.to_string()
    }
}

fn make_step_id(trajectory_id: &str, step_index: u32) -> String {
    if trajectory_id.is_empty() {
        step_index.to_string()
    } else {
        format!("{trajectory_id}:{step_index}")
    }
}

// =============================================================================
// LocalConnectionStep — extends Step with connection-specific fields
// =============================================================================

/// Connection-specific step for LocalConnection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalConnectionStep {
    #[serde(flatten)]
    pub step: Step,
    #[serde(default)]
    pub cascade_id: String,
    #[serde(default)]
    pub trajectory_id: String,
    #[serde(default)]
    pub http_code: u32,
}

impl LocalConnectionStep {
    /// Creates a LocalConnectionStep from a StepUpdate wire message.
    pub fn from_wire(_su: &crate::connections::wire_types::localharness::StepUpdate) -> Self {
        Self {
            step: crate::types::Step::default(),
            cascade_id: "".to_string(),
            trajectory_id: "".to_string(),
            http_code: 0,
        }
    }
}

// =============================================================================
// LocalConnection
// =============================================================================

#[derive(Debug, Default)]
struct StepTracker {
    state: String,
    handled_requests: HashSet<String>,
}

impl StepTracker {
    fn update_state(&mut self, new_state: &str) {
        if self.state == "STATE_WAITING_FOR_USER" && new_state != "STATE_WAITING_FOR_USER" {
            self.handled_requests.clear();
        }
        self.state = new_state.to_string();
    }

    fn mark_handled(&mut self, request_type: &str) -> bool {
        self.handled_requests.insert(request_type.to_string())
    }
}

struct PendingCallValue {
    tool_call: crate::types::ToolCall,
}

/// Internal queue item for the WebSocket reader task.
enum QueueItem {
    Step(LocalConnectionStep),
    Idle,
    Close,
    Error(AntigravityConnectionError),
}

/// Connection to the Go-based local harness.
pub struct LocalConnection {
    ws_writer: Arc<Mutex<tokio::sync::mpsc::UnboundedSender<Vec<u8>>>>,
    step_rx: Mutex<tokio::sync::mpsc::UnboundedReceiver<QueueItem>>,
    is_idle: Arc<tokio::sync::Notify>,
    idle_flag: Arc<std::sync::atomic::AtomicBool>,
    cascade_id: Arc<RwLock<Option<String>>>,
    reader_handle: Mutex<Option<JoinHandle<()>>>,
    process: Mutex<Option<Child>>,
    disconnecting: std::sync::atomic::AtomicBool,
    current_turn_context: Arc<Mutex<Option<TurnContext>>>,
    step_trackers: Arc<Mutex<std::collections::HashMap<(String, u32), StepTracker>>>,
    active_subagent_ids: Arc<Mutex<std::collections::HashSet<String>>>,
    subagent_responses: Arc<Mutex<std::collections::HashMap<String, String>>>,
    parent_idle: Arc<std::sync::atomic::AtomicBool>,
    stderr_lines: Arc<Mutex<std::collections::VecDeque<String>>>,
    pending_builtin_tool_calls:
        Arc<Mutex<std::collections::HashMap<(String, u32), PendingCallValue>>>,

    // Tools and hooks
    pub tool_runner: Option<Arc<ToolRunner>>,
    pub hook_runner: Option<Arc<HookRunner>>,
}

fn _extract_tool_result(
    _su: &crate::connections::wire_types::localharness::StepUpdate,
) -> Option<serde_json::Value> {
    None
}

impl LocalConnection {
    fn _to_proto_input_content(
        content: &crate::types::ContentPrimitive,
    ) -> crate::connections::wire_types::localharness::user_input::Part {
        match content {
            crate::types::ContentPrimitive::Text(t) => {
                crate::connections::wire_types::localharness::user_input::Part {
                    part: Some(
                        crate::connections::wire_types::localharness::user_input::part::Part::Text(
                            t.clone(),
                        ),
                    ),
                }
            }
            crate::types::ContentPrimitive::Media(m) => {
                crate::connections::wire_types::localharness::user_input::Part {
                    part: Some(
                        crate::connections::wire_types::localharness::user_input::part::Part::Media(
                            crate::connections::wire_types::localharness::user_input::Media {
                                mime_type: Some(m.mime_type.clone()),
                                description: m.description.clone(),
                                data: Some(m.data.clone()),
                            },
                        ),
                    ),
                }
            }
        }
    }

    fn _get_turn_context(&self) -> TurnContext {
        if let Some(hr) = &self.hook_runner {
            TurnContext::new(&hr.session_context.read().unwrap())
        } else {
            // Provide a dummy context if there's no hook runner
            TurnContext {
                ctx: crate::hooks::HookContext::new(),
                turn_id: "".to_string(),
                step_count: 0,
                metadata: HashMap::new(),
            }
        }
    }

    fn start_stderr_reader(
        mut stderr: tokio::process::ChildStderr,
        lines: Arc<Mutex<VecDeque<String>>>,
    ) {
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(&mut stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let mut l = lines.lock().await;
                if l.len() >= 100 {
                    l.pop_front();
                }
                l.push_back(line.clone());
                info!("harness stderr: {}", line);
            }
        });
    }

    async fn handle_question_request(
        &self,
        _su: crate::connections::wire_types::localharness::StepUpdate,
    ) {
        tracing::warn!("handle_question_request not fully implemented for protobuf yet");
    }

    async fn handle_tool_confirmation_request(
        &self,
        su: crate::connections::wire_types::localharness::StepUpdate,
    ) {
        let resp = crate::connections::wire_types::localharness::ToolConfirmation {
            trajectory_id: su.trajectory_id.clone(),
            step_index: su.step_index,
            accepted: Some(true),
        };
        let event = crate::connections::wire_types::localharness::InputEvent {
            event: Some(input_event::Event::ToolConfirmation(resp)),
        };
        use prost::Message;
        let json = event.encode_to_vec();
        let _ = self.ws_writer.lock().await.send(json);
    }

    async fn handle_tool_call(
        &self,
        wire_tc: crate::connections::wire_types::localharness::ToolCall,
    ) {
        // Step 1: Parse (pure)
        let tc = match tool_core::parse_wire_tool_call(
            wire_tc.id.clone(),
            wire_tc.name.clone(),
            wire_tc.arguments_json.clone(),
        ) {
            Ok(tc) => tc,
            Err(e) => {
                warn!("Failed to parse wire tool call: {}", e);
                return;
            }
        };

        // Step 2: Policy check (async, may deny)
        let (allowed, deny_msg, op_ctx) = self.check_tool_policy(&tc).await;

        if !allowed {
            let msg = tool_core::effective_deny_message(&deny_msg);
            let denial = tool_core::build_denial_result(&tc, msg);
            let _ = self.send_tool_results(vec![denial]).await;
            return;
        }

        // Step 3: Execute tool + post-process hooks (async)
        let result = self.execute_and_post_process(&tc, op_ctx).await;

        // Step 4: Send result
        if let Some(result) = result {
            let _ = self.send_tool_results(vec![result]).await;
        }
    }

    /// Encapsulates the hook_runner pre-tool-call policy check.
    ///
    /// Returns `(allowed, deny_message, optional_op_context)`.
    async fn check_tool_policy(
        &self,
        tc: &crate::types::ToolCall,
    ) -> (bool, String, Option<OperationContext>) {
        let hr = match &self.hook_runner {
            Some(hr) => hr,
            None => return (true, String::new(), None),
        };

        let mut guard = self.current_turn_context.lock().await;
        if guard.is_none() {
            *guard = Some(self._get_turn_context());
        }

        let (res, op_ctx) = hr
            .dispatch_pre_tool_call(guard.as_ref().unwrap(), tc)
            .await;

        (res.allow, res.message.clone(), Some(op_ctx))
    }

    /// Encapsulates tool execution and post-execution hook dispatch.
    ///
    /// Runs the tool via `tool_runner`, then dispatches either `on_tool_error`
    /// (with `resolve_tool_result` for recovery) or `post_tool_call` hooks.
    async fn execute_and_post_process(
        &self,
        tc: &crate::types::ToolCall,
        op_ctx: Option<OperationContext>,
    ) -> Option<ToolResult> {
        let tr = match &self.tool_runner {
            Some(tr) => tr,
            None => {
                warn!("Received tool call but no tool runner is configured");
                return None;
            }
        };

        let results = tr.process_tool_calls(&[tc.clone()]).await;
        let mut result = results.into_iter().next()?;
        result.id = tc.id.clone();

        if let Some(hr) = &self.hook_runner {
            let mut op_context =
                op_ctx.unwrap_or_else(|| OperationContext::new(&self._get_turn_context()));

            if result.error.is_some() {
                let e = std::io::Error::other(result.error.clone().unwrap());
                let (rec_res, rec_val) = hr
                    .dispatch_on_tool_error(&mut op_context, Box::new(e))
                    .await;
                let recovery = if rec_res.allow { rec_val } else { None };
                result = tool_core::resolve_tool_result(result, recovery);
            } else {
                hr.dispatch_post_tool_call(&mut op_context, &result).await;
            }
        }

        Some(result)
    }
}


#[async_trait]
impl Connection for LocalConnection {
    fn is_idle(&self) -> bool {
        self.idle_flag.load(std::sync::atomic::Ordering::Acquire)
    }

    fn conversation_id(&self) -> &str {
        // Use async_conversation_id instead
        ""
    }

    async fn send(&self, prompt: Option<Content>) -> Result<(), AntigravityConnectionError> {
        self.idle_flag
            .store(false, std::sync::atomic::Ordering::Release);

        if let Some(hr) = &self.hook_runner {
            let (res, turn_ctx) = hr.dispatch_pre_turn(prompt.as_ref()).await;
            *self.current_turn_context.lock().await = Some(turn_ctx);
            if !res.allow {
                warn!("Turn denied by hook: {:?}", res.message);
                self.idle_flag
                    .store(true, std::sync::atomic::Ordering::Release);
                self.is_idle.notify_waiters();
                return Ok(());
            }
        }

        let mut ui = crate::connections::wire_types::localharness::UserInput::default();
        if let Some(prompt_content) = prompt {
            match prompt_content {
                Content::Single(p) => {
                    ui.parts.push(Self::_to_proto_input_content(&p));
                }
                Content::Multiple(multi) => {
                    for p in multi {
                        ui.parts.push(Self::_to_proto_input_content(&p));
                    }
                }
            }
        }

        let event = crate::connections::wire_types::localharness::InputEvent {
            event: Some(input_event::Event::ComplexUserInput(ui)),
        };

        use prost::Message;
        let mut json = Vec::new();
        event.encode(&mut json).unwrap();

        let lock = self.ws_writer.lock().await;
        lock.send(json)
            .map_err(|e: tokio::sync::mpsc::error::SendError<Vec<u8>>| {
                AntigravityConnectionError {
                    message: e.to_string(),
                }
            })?;

        Ok(())
    }

    fn receive_steps(&self) -> Pin<Box<dyn Stream<Item = Step> + Send + '_>> {
        Box::pin(futures::stream::unfold((), move |()| async move {
            let mut rx = self.step_rx.lock().await;
            loop {
                match rx.recv().await {
                    Some(QueueItem::Step(step)) => return Some((step.step, ())),
                    Some(_) => continue,
                    None => return None,
                }
            }
        }))
    }

    async fn disconnect(&self) -> Result<(), AntigravityConnectionError> {
        Ok(())
    }

    async fn cancel(&self) -> Result<(), AntigravityConnectionError> {
        let event = crate::connections::wire_types::localharness::InputEvent {
            event: Some(input_event::Event::HaltRequest(true)),
        };
        use prost::Message;
        let mut json = Vec::new();
        event.encode(&mut json).unwrap();

        let lock = self.ws_writer.lock().await;
        let _ = lock.send(json);
        Ok(())
    }

    async fn delete(&self) -> Result<(), AntigravityConnectionError> {
        Ok(())
    }

    async fn signal_idle(&self) -> Result<(), AntigravityConnectionError> {
        self.idle_flag
            .store(true, std::sync::atomic::Ordering::Release);
        self.is_idle.notify_waiters();
        Ok(())
    }

    async fn wait_for_idle(&self) -> Result<(), AntigravityConnectionError> {
        while !self.is_idle() {
            self.is_idle.notified().await;
        }
        Ok(())
    }

    async fn wait_for_wakeup(&self, timeout_secs: f64) -> Result<bool, AntigravityConnectionError> {
        if !self.is_idle() {
            return Ok(true);
        }
        let dur = std::time::Duration::from_secs_f64(timeout_secs);
        if let Ok(_) = tokio::time::timeout(dur, self.is_idle.notified()).await {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn send_tool_results(
        &self,
        results: Vec<ToolResult>,
    ) -> Result<(), AntigravityConnectionError> {
        for res in results {
            let event = crate::connections::wire_types::localharness::InputEvent {
                event: Some(input_event::Event::ToolResponse(
                    crate::connections::wire_types::localharness::ToolResponse {
                        id: res.id.clone(),
                        response_json: res
                            .result
                            .as_ref()
                            .map(|v| serde_json::to_string(v).unwrap_or_default()),
                        supplemental_media: vec![],
                        response: None,
                    },
                )),
            };
            use prost::Message;
            let mut json = Vec::new();
            event.encode(&mut json).unwrap();
            let lock = self.ws_writer.lock().await;
            let _ = lock.send(json);
        }
        Ok(())
    }

    async fn send_trigger_notification(
        &self,
        _content: &str,
    ) -> Result<(), AntigravityConnectionError> {
        Ok(())
    }
}

pub struct LocalConnectionStrategy {
    config: crate::connections::config::LocalAgentConfig,
    tool_runner: Option<Arc<crate::tools::ToolRunner>>,
    hook_runner: Option<Arc<crate::hooks::HookRunner>>,
}

impl LocalConnectionStrategy {
    pub fn new(config: crate::connections::config::LocalAgentConfig) -> Self {
        Self {
            config,
            tool_runner: None,
            hook_runner: None,
        }
    }

    pub fn with_tool_runner(mut self, runner: Arc<crate::tools::ToolRunner>) -> Self {
        self.tool_runner = Some(runner);
        self
    }

    pub fn with_hook_runner(mut self, runner: Arc<crate::hooks::HookRunner>) -> Self {
        self.hook_runner = Some(runner);
        self
    }
}

#[async_trait]
impl ConnectionStrategy for LocalConnectionStrategy {
    fn connect(&self) -> Result<Arc<dyn Connection>, crate::types::AntigravityConnectionError> {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let (_step_tx, step_rx) = tokio::sync::mpsc::unbounded_channel();
        Ok(Arc::new(LocalConnection {
            ws_writer: Arc::new(Mutex::new(tx)),
            step_rx: Mutex::new(step_rx),
            is_idle: Arc::new(tokio::sync::Notify::new()),
            idle_flag: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            cascade_id: Arc::new(RwLock::new(None)),
            reader_handle: Mutex::new(None),
            process: Mutex::new(None),
            disconnecting: std::sync::atomic::AtomicBool::new(false),
            current_turn_context: Arc::new(Mutex::new(None)),
            step_trackers: Arc::new(Mutex::new(std::collections::HashMap::new())),
            active_subagent_ids: Arc::new(Mutex::new(std::collections::HashSet::new())),
            subagent_responses: Arc::new(Mutex::new(std::collections::HashMap::new())),
            parent_idle: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            stderr_lines: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            pending_builtin_tool_calls: Arc::new(Mutex::new(std::collections::HashMap::new())),
            tool_runner: self.tool_runner.clone(),
            hook_runner: self.hook_runner.clone(),
        }))
    }

    async fn start(&mut self) -> Result<(), crate::types::AntigravityConnectionError> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), crate::types::AntigravityConnectionError> {
        Ok(())
    }
}
