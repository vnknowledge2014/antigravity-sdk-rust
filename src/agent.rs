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

//! Layer 1 API for the Antigravity SDK.
//!
//! The [`Agent`] struct is the high-level entry point for creating and
//! interacting with AI agents. It orchestrates the HookRunner, ToolRunner,
//! TriggerRunner, McpBridge, and Connection lifecycle.

use std::sync::Arc;
use tracing::{info, warn};

use crate::connections::{ConnectionStrategy, LocalAgentConfig, LocalConnectionStrategy};
use crate::conversation::Conversation;
use crate::core::agent_core;
use crate::hooks::{HookRunner, hook_runner::AnyHook, policy};
use crate::mcp::McpBridge;
use crate::tools::{RegisteredTool, ToolRunner, tool_context::ToolContext};
use crate::triggers::{TriggerRunner, triggers::Trigger};
use crate::types::{BuiltinTools, ChatResponse, Content, McpServerConfig};

/// Full configuration for the Agent.
#[derive(Default)]
pub struct AgentArgs {
    pub local_config: LocalAgentConfig,
    pub policies: Vec<policy::Policy>,
    pub hooks: Vec<AnyHook>,
    pub triggers: Vec<Trigger>,
    pub mcp_servers: Vec<McpServerConfig>,
    pub tools: Vec<RegisteredTool>,
}


/// High-level Agent API for simplified interaction.
pub struct Agent {
    config: AgentArgs,
    hook_runner: Option<Arc<HookRunner>>,
    tool_runner: Option<Arc<ToolRunner>>,
    trigger_runner: Option<TriggerRunner>,
    mcp_bridge: Option<McpBridge>,
    conversation: Option<Conversation>,
    strategy: Option<Box<dyn ConnectionStrategy>>,
    pending_hooks: Vec<AnyHook>,
    pending_triggers: Vec<Trigger>,
    /// Current lifecycle phase — replaces the old `started: bool`.
    phase: agent_core::AgentPhase,
    /// Append-only event log for debugging and replay.
    events: Vec<agent_core::AgentEvent>,
}

impl Agent {
    /// Creates a new Agent (not yet started).
    pub fn new(mut config: AgentArgs) -> Self {
        let pending_hooks = std::mem::take(&mut config.hooks);
        let pending_triggers = std::mem::take(&mut config.triggers);
        Self {
            config,
            hook_runner: None,
            tool_runner: None,
            trigger_runner: None,
            mcp_bridge: None,
            conversation: None,
            strategy: None,
            pending_hooks,
            pending_triggers,
            phase: agent_core::AgentPhase::Created,
            events: Vec::new(),
        }
    }

    /// True if the agent backend is running.
    pub fn is_started(&self) -> bool {
        self.phase == agent_core::AgentPhase::Running
    }

    /// Returns the current lifecycle phase.
    pub fn phase(&self) -> &agent_core::AgentPhase {
        &self.phase
    }

    /// Returns the event log for debugging and replay.
    pub fn events(&self) -> &[agent_core::AgentEvent] {
        &self.events
    }

    /// The conversation ID, if the agent has started.
    pub fn conversation_id(&self) -> Option<&str> {
        self.conversation.as_ref().map(|c| c.conversation_id())
    }

    /// Registers a hook.
    pub fn register_hook(&mut self, hook: AnyHook) {
        if let Some(_arc) = &mut self.hook_runner {
            // Note: because hook_runner is shared with ConnectionStrategy via Arc,
            // we cannot mutate it after start without an RwLock.
            // Python SDK allows this, but idiomatic Rust requires interior mutability.
            warn!("Cannot register hooks after start when HookRunner is shared.");
        } else {
            self.pending_hooks.push(hook);
        }
    }

    pub fn hook_runner_mut(&mut self) -> Option<&mut HookRunner> {
        self.hook_runner.as_mut().and_then(Arc::get_mut)
    }

    pub fn tool_runner_mut(&mut self) -> Option<&mut ToolRunner> {
        self.tool_runner.as_mut().and_then(Arc::get_mut)
    }

    pub fn conversation(&self) -> Option<&Conversation> {
        self.conversation.as_ref()
    }

    pub fn conversation_mut(&mut self) -> Option<&mut Conversation> {
        self.conversation.as_mut()
    }

    /// Registers a trigger.
    pub fn register_trigger(&mut self, trigger: Trigger) -> Result<(), String> {
        if self.is_started() {
            return Err("Cannot register triggers after the agent has started.".to_string());
        }
        self.pending_triggers.push(trigger);
        Ok(())
    }

    /// Sends a chat message to the agent.
    pub async fn chat(
        &mut self,
        prompt: impl Into<Content>,
    ) -> Result<ChatResponse<'_>, Box<dyn std::error::Error + Send + Sync>> {
        if !self.is_started() {
            return Err("Agent must be started before chatting.".into());
        }
        if let Some(conv) = &self.conversation {
            Ok(conv.chat(Some(prompt.into())).await?)
        } else {
            Err("Conversation not initialized.".into())
        }
    }

    /// Starts the agent backend and prepares for connections.
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.phase == agent_core::AgentPhase::Running {
            return Ok(());
        }
        info!("Starting Agent session");
        self.phase = agent_core::AgentPhase::Starting;

        let mut hook_runner = self.build_hook_runner()?;
        let initial_hook_count = hook_runner.has_hooks() as usize;
        self.events.push(agent_core::AgentEvent::HookRunnerCreated {
            hook_count: initial_hook_count,
        });

        self.validate_and_apply_policies(&mut hook_runner)?;
        let hr_arc = Arc::new(hook_runner);

        let mut mcp_bridge = self.connect_mcp_servers().await?;
        let tr_arc = self.build_tool_runner(mcp_bridge.as_mut())?;
        let connection = self.establish_connection(&hr_arc, &tr_arc).await?;

        self.events.push(agent_core::AgentEvent::ConnectionEstablished {
            conversation_id: connection.conversation_id().to_string(),
        });

        self.create_conversation_and_triggers(&connection);
        self.wire_tool_context(&tr_arc, &connection).await;
        self.finalize_start(hr_arc, tr_arc, mcp_bridge);

        self.phase = agent_core::AgentPhase::Running;
        self.events.push(agent_core::AgentEvent::Started);
        Ok(())
    }

    /// Drains pending hooks into a new `HookRunner`.
    fn build_hook_runner(
        &mut self,
    ) -> Result<HookRunner, Box<dyn std::error::Error + Send + Sync>> {
        let mut hook_runner = HookRunner::new();
        for hook in std::mem::take(&mut self.pending_hooks) {
            hook_runner.register_hook(hook);
        }
        Ok(hook_runner)
    }

    /// Pure validation via `agent_core`, then registers the policy-enforce hook.
    fn validate_and_apply_policies(
        &mut self,
        hr: &mut HookRunner,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let active_tools =
            agent_core::compute_active_tools(&self.config.local_config.capabilities);
        let has_mcp = !self.config.mcp_servers.is_empty();
        let active_policies = std::mem::take(&mut self.config.policies);

        agent_core::validate_safety(
            &active_tools,
            has_mcp,
            active_policies.len(),
            hr.has_pre_tool_call_decide_hooks(),
        )?;

        if !active_policies.is_empty() {
            hr.register_hook(crate::hooks::hook_runner::AnyHook::PreToolCallDecide(
                Box::new(policy::enforce(active_policies)),
            ));
        }
        Ok(())
    }

    /// Connects to configured MCP servers, returning an optional bridge.
    async fn connect_mcp_servers(
        &mut self,
    ) -> Result<Option<McpBridge>, Box<dyn std::error::Error + Send + Sync>> {
        if self.config.mcp_servers.is_empty() {
            return Ok(None);
        }
        info!("Connecting to MCP servers...");
        let mut bridge = McpBridge::new();
        for server_cfg in &self.config.mcp_servers {
            match server_cfg {
                McpServerConfig::Stdio(s) => bridge.connect_stdio(&s.command, &s.args).await?,
                McpServerConfig::Sse(s) => {
                    bridge.connect_sse(&s.url, s.headers.as_ref()).await?
                }
                McpServerConfig::Http(_) => {
                    return Err("HTTP MCP servers not implemented".into());
                }
            }
        }
        Ok(Some(bridge))
    }

    /// Registers user tools and MCP-discovered tools into a `ToolRunner`.
    fn build_tool_runner(
        &mut self,
        mcp: Option<&mut McpBridge>,
    ) -> Result<Arc<ToolRunner>, Box<dyn std::error::Error + Send + Sync>> {
        let mut tool_runner = ToolRunner::new();
        let all_tools = std::mem::take(&mut self.config.tools);
        for tool in all_tools {
            tool_runner.register(tool).map_err(|e| e.to_string())?;
        }
        if let Some(bridge) = mcp {
            for mcp_tool in bridge.take_tools() {
                tool_runner
                    .register_mcp_tool(mcp_tool)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(Arc::new(tool_runner))
    }

    /// Starts the connection strategy and returns the live connection.
    async fn establish_connection(
        &mut self,
        hr: &Arc<HookRunner>,
        tr: &Arc<ToolRunner>,
    ) -> Result<Arc<dyn crate::connections::Connection>, Box<dyn std::error::Error + Send + Sync>>
    {
        let mut strategy = LocalConnectionStrategy::new(self.config.local_config.clone())
            .with_tool_runner(tr.clone())
            .with_hook_runner(hr.clone());

        strategy.start().await?;
        let connection = strategy.connect()?;
        self.strategy = Some(Box::new(strategy));
        Ok(connection)
    }

    /// Creates the conversation and wires up trigger runners.
    fn create_conversation_and_triggers(
        &mut self,
        conn: &Arc<dyn crate::connections::Connection>,
    ) {
        self.conversation = Some(Conversation::new(conn.clone()));

        if !self.pending_triggers.is_empty() {
            let mut tr = TriggerRunner::new(std::mem::take(&mut self.pending_triggers));
            tr.start(conn.clone());
            self.trigger_runner = Some(tr);
        }
    }

    /// Sets the tool context so tools can interact with the connection.
    async fn wire_tool_context(
        &self,
        tr: &Arc<ToolRunner>,
        conn: &Arc<dyn crate::connections::Connection>,
    ) {
        let tool_ctx = ToolContext::new(conn.clone());
        tr.set_context(tool_ctx).await;
    }

    /// Stores shared handles on `self`.
    fn finalize_start(
        &mut self,
        hr: Arc<HookRunner>,
        tr: Arc<ToolRunner>,
        mcp: Option<McpBridge>,
    ) {
        self.hook_runner = Some(hr);
        self.tool_runner = Some(tr);
        self.mcp_bridge = mcp;
    }

    /// Stops the agent backend and cleans up resources.
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.phase != agent_core::AgentPhase::Running {
            return Ok(());
        }
        info!("Stopping Agent session");
        self.phase = agent_core::AgentPhase::Stopping;

        if let Some(mut tr) = self.trigger_runner.take() {
            tr.stop().await;
        }

        self.conversation = None;

        if let Some(mut bridge) = self.mcp_bridge.take() {
            bridge.stop().await;
        }

        self.strategy = None;
        self.tool_runner = None;
        self.hook_runner = None;
        self.phase = agent_core::AgentPhase::Stopped;
        self.events.push(agent_core::AgentEvent::Stopped);
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent_core::AgentPhase;

    #[test]
    fn test_agent_lifecycle_unit() {
        let agent = Agent::new(AgentArgs::default());
        assert!(!agent.is_started());
    }

    #[test]
    fn test_agent_initial_phase() {
        let agent = Agent::new(AgentArgs::default());
        assert_eq!(*agent.phase(), AgentPhase::Created);
    }

    #[test]
    fn test_agent_events_initially_empty() {
        let agent = Agent::new(AgentArgs::default());
        assert!(agent.events().is_empty());
    }

    #[test]
    fn test_agent_phase_accessor() {
        let agent = Agent::new(AgentArgs::default());
        assert_eq!(*agent.phase(), AgentPhase::Created);
        assert!(!agent.is_started());
    }

    #[test]
    fn test_agent_conversation_id_before_start() {
        let agent = Agent::new(AgentArgs::default());
        assert!(agent.conversation_id().is_none());
    }

    #[test]
    fn test_agent_register_hook_before_start() {
        let mut agent = Agent::new(AgentArgs::default());
        // Use a PreToolCallDecide hook which has a simpler type
        let policy = crate::hooks::policy::allow("*");
        let hook = AnyHook::PreToolCallDecide(
            Box::new(crate::hooks::policy::enforce(vec![policy]))
        );
        agent.register_hook(hook);
        // Should not panic — hooks are queued
    }

    #[test]
    fn test_agent_register_trigger_before_start() {
        let mut agent = Agent::new(AgentArgs::default());
        let trigger = Box::new(|_ctx: std::sync::Arc<crate::triggers::triggers::TriggerContext>| {
            Box::pin(async {}) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        });
        let result = agent.register_trigger(trigger);
        assert!(result.is_ok());
    }
}
