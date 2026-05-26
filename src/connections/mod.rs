use std::sync::Arc;
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

// Connection abstraction layer for the Antigravity SDK.
//
// A [`Connection`] is the SDK's public interface for interacting with an agent
// backend, regardless of where the agent runs. Layer 2 APIs (Conversation,
// AgentConfig) depend ONLY on this interface — never on transport details.
//
// A [`ConnectionStrategy`] knows how to establish a Connection for a specific
// backend type and how to tear it down.

pub mod config;
pub mod local;
pub mod wire_types;

// Re-exports for ergonomic usage
pub use config::LocalAgentConfig;
pub use local::{LocalConnection, LocalConnectionStrategy};

use crate::types::{
    AntigravityConnectionError, CapabilitiesConfig, Content, McpServerConfig, Step,
    SystemInstructions, ToolResult,
};
use async_trait::async_trait;
use futures::Stream;
use std::any::Any;
use std::pin::Pin;

/// Abstract base for agent configuration.
///
/// Each ConnectionStrategy defines a concrete implementation with the config
/// fields it needs. Agent introspects the config type to auto-dispatch to
/// the correct strategy factory.
pub trait AgentConfig: Send + Sync {
    /// Creates the ConnectionStrategy for this config.
    fn create_strategy(
        &self,
        tool_runner: Box<dyn Any + Send + Sync>,
        hook_runner: Box<dyn Any + Send + Sync>,
    ) -> Box<dyn ConnectionStrategy>;

    // Common config fields — provided by all implementations.
    fn system_instructions(&self) -> Option<&SystemInstructions>;
    fn capabilities(&self) -> &CapabilitiesConfig;
    fn tools(&self) -> &[Box<dyn Any + Send + Sync>];
    fn policies(&self) -> &[Box<dyn Any + Send + Sync>];
    fn hooks(&self) -> &[Box<dyn Any + Send + Sync>];
    fn triggers(&self) -> &[Box<dyn Any + Send + Sync>];
    fn mcp_servers(&self) -> &[McpServerConfig];
    fn workspaces(&self) -> &[String];
    fn conversation_id(&self) -> Option<&str>;
    fn save_dir(&self) -> Option<&str>;
    fn app_data_dir(&self) -> Option<&str>;
    fn response_schema(&self) -> Option<&str>;
    fn skills_paths(&self) -> &[String];
}

/// A live session with an agent backend.
///
/// This is the common contract that all connection types implement.
/// Layer 2 APIs depend only on this interface.
#[async_trait]
pub trait Connection: Send + Sync {
    /// Returns `true` if the connection is idle and ready for input.
    fn is_idle(&self) -> bool {
        true
    }

    /// Returns the conversation identifier, or empty string if unset.
    fn conversation_id(&self) -> &str {
        ""
    }

    /// Sends a prompt to the agent.
    async fn send(&self, prompt: Option<Content>) -> Result<(), AntigravityConnectionError>;

    /// Receives steps as they complete from the agent.
    ///
    /// Returns a stream of Step objects representing agent actions.
    fn receive_steps(&self) -> Pin<Box<dyn Stream<Item = Step> + Send + '_>>;

    /// Disconnects the session and releases resources.
    async fn disconnect(&self) -> Result<(), AntigravityConnectionError> {
        Ok(())
    }

    /// Cancels the current turn in progress.
    async fn cancel(&self) -> Result<(), AntigravityConnectionError> {
        Ok(())
    }

    /// Deletes this connection and all associated state from the backend.
    async fn delete(&self) -> Result<(), AntigravityConnectionError> {
        Ok(())
    }

    /// Signals that the connection is idle and ready to receive input.
    async fn signal_idle(&self) -> Result<(), AntigravityConnectionError> {
        Ok(())
    }

    /// Blocks until the connection becomes idle.
    async fn wait_for_idle(&self) -> Result<(), AntigravityConnectionError> {
        Ok(())
    }

    /// Blocks until the connection wakes up or the timeout is reached.
    async fn wait_for_wakeup(
        &self,
        _timeout_secs: f64,
    ) -> Result<bool, AntigravityConnectionError> {
        Ok(false)
    }

    /// Sends tool execution results back to the agent.
    async fn send_tool_results(
        &self,
        _results: Vec<ToolResult>,
    ) -> Result<(), AntigravityConnectionError> {
        Ok(())
    }

    /// Sends a trigger message to the agent.
    async fn send_trigger_notification(
        &self,
        content: &str,
    ) -> Result<(), AntigravityConnectionError>;
}

/// Strategy for establishing a Connection to an agent backend.
///
/// Each backend type provides its own ConnectionStrategy implementation
/// that handles process management, transport setup, authentication, and
/// health checking.
#[async_trait]
pub trait ConnectionStrategy: Send + Sync {
    /// Returns the established Connection.
    fn connect(&self) -> Result<Arc<dyn Connection>, AntigravityConnectionError>;

    /// Starts the backend and prepares for connections.
    async fn start(&mut self) -> Result<(), AntigravityConnectionError>;

    /// Tears down the backend and releases all resources.
    async fn stop(&mut self) -> Result<(), AntigravityConnectionError>;
}
