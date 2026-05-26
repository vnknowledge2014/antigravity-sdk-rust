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
    started: bool,
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
            started: false,
        }
    }

    /// True if the agent backend is running.
    pub fn is_started(&self) -> bool {
        self.started
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
        if self.started {
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
        if !self.started {
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
        if self.started {
            return Ok(());
        }
        info!("Starting Agent session");

        let mut hook_runner = HookRunner::new();
        for hook in std::mem::take(&mut self.pending_hooks) {
            hook_runner.register_hook(hook);
        }

        let cfg = &self.config.local_config.capabilities;
        let read_only_tools = BuiltinTools::read_only();
        let active_tools: std::collections::HashSet<BuiltinTools> =
            if let Some(ref enabled) = cfg.enabled_tools {
                enabled.iter().copied().collect()
            } else if let Some(ref disabled) = cfg.disabled_tools {
                let mut all = BuiltinTools::all_tools()
                    .into_iter()
                    .collect::<std::collections::HashSet<_>>();
                for d in disabled {
                    all.remove(d);
                }
                all
            } else {
                BuiltinTools::all_tools().into_iter().collect()
            };

        let mut has_write_tools = false;
        for t in &active_tools {
            if !read_only_tools.contains(t) {
                has_write_tools = true;
                break;
            }
        }

        let has_mcp_servers = !self.config.mcp_servers.is_empty();
        let has_tool_decide_hook = hook_runner.has_pre_tool_call_decide_hooks();
        let active_policies = std::mem::take(&mut self.config.policies);

        if (has_write_tools || has_mcp_servers)
            && active_policies.is_empty()
            && !has_tool_decide_hook
        {
            return Err(
                "Write tools or MCP servers are enabled without a safety policy. \
                        Add policies=[policy::allow_all()] to approve all tool calls, \
                        or policies=[policy::deny_all(), policy::allow(\"tool_name\")] \
                        to selectively allow specific tools."
                    .into(),
            );
        }

        if !active_policies.is_empty() {
            hook_runner.register_hook(crate::hooks::hook_runner::AnyHook::PreToolCallDecide(
                Box::new(policy::enforce(active_policies)),
            ));
        }

        let hr_arc = Arc::new(hook_runner);
        self.hook_runner = Some(hr_arc.clone());

        if !self.config.mcp_servers.is_empty() {
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
            self.mcp_bridge = Some(bridge);
        }

        let mut tool_runner = ToolRunner::new();
        let all_tools = std::mem::take(&mut self.config.tools);
        for tool in all_tools {
            tool_runner.register(tool).map_err(|e| e.to_string())?;
        }
        if let Some(bridge) = &mut self.mcp_bridge {
            for mcp_tool in bridge.take_tools() {
                tool_runner
                    .register_mcp_tool(mcp_tool)
                    .map_err(|e| e.to_string())?;
            }
        }
        let tr_arc = Arc::new(tool_runner);
        self.tool_runner = Some(tr_arc.clone());

        let mut strategy = LocalConnectionStrategy::new(self.config.local_config.clone())
            .with_tool_runner(tr_arc.clone())
            .with_hook_runner(hr_arc.clone());

        strategy.start().await?;
        let connection = strategy.connect()?;
        self.strategy = Some(Box::new(strategy));

        let conversation = Conversation::new(connection.clone());
        self.conversation = Some(conversation);

        if !self.pending_triggers.is_empty() {
            let mut tr = TriggerRunner::new(std::mem::take(&mut self.pending_triggers));
            tr.start(connection.clone());
            self.trigger_runner = Some(tr);
        }

        let tool_ctx = ToolContext::new(connection.clone());
        tr_arc.set_context(tool_ctx).await;

        self.started = true;
        Ok(())
    }

    /// Stops the agent backend and cleans up resources.
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.started {
            return Ok(());
        }
        info!("Stopping Agent session");

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
        self.started = false;
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_lifecycle_unit() {
        let agent = Agent::new(AgentArgs::default());
        assert!(!agent.is_started());
    }
}
