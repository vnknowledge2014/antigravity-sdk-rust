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

//! Tool execution context for the Antigravity SDK.
//!
//! Corresponds to Python's `tools/tool_context.py`.

use crate::connections::Connection;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Context injected into tools that request it.
///
/// Provides access to conversation-level state such as the conversation ID,
/// idle state, and a key-value store.
pub struct ToolContext {
    connection: Arc<dyn Connection>,
    store: HashMap<String, Value>,
}

impl ToolContext {
    /// Creates a new ToolContext wrapping a Connection.
    pub fn new(connection: Arc<dyn Connection>) -> Self {
        Self {
            connection,
            store: HashMap::new(),
        }
    }

    /// Returns the conversation identifier.
    pub fn conversation_id(&self) -> String {
        self.connection.conversation_id().to_string()
    }

    /// Returns True if the connection is idle.
    pub fn is_idle(&self) -> bool {
        self.connection.is_idle()
    }

    /// Sends a trigger message to the agent via the connection.
    pub async fn send(
        &self,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.connection
            .send_trigger_notification(message)
            .await
            .map_err(|e| Box::new(e) as _)
    }

    /// Gets a value from the state store.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.store.get(key)
    }

    /// Sets a value in the state store.
    pub fn set(&mut self, key: String, value: Value) {
        self.store.insert(key, value);
    }

    /// Returns true if the store contains the given key.
    pub fn has(&self, key: &str) -> bool {
        self.store.contains_key(key)
    }

    /// Removes a key from the store and returns its value if present.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.store.remove(key)
    }

    /// Returns the number of items in the store.
    pub fn store_len(&self) -> usize {
        self.store.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AntigravityConnectionError, Content, Step};
    use async_trait::async_trait;
    use futures::Stream;
    use std::pin::Pin;
    use std::sync::Mutex;

    struct MockConnection {
        conversation_id: String,
        is_idle: bool,
        sent_messages: Mutex<Vec<String>>,
    }

    impl MockConnection {
        fn new(conversation_id: &str, is_idle: bool) -> Self {
            Self {
                conversation_id: conversation_id.to_string(),
                is_idle,
                sent_messages: Mutex::new(Vec::new()),
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
            content: &str,
        ) -> Result<(), AntigravityConnectionError> {
            self.sent_messages.lock().unwrap().push(content.to_string());
            Ok(())
        }
    }

    #[test]
    fn test_conversation_id() {
        let conn = Arc::new(MockConnection::new("conv-123", false));
        let ctx = ToolContext::new(conn);
        assert_eq!(ctx.conversation_id(), "conv-123");
    }

    #[test]
    fn test_is_idle_true() {
        let conn = Arc::new(MockConnection::new("conv-1", true));
        let ctx = ToolContext::new(conn);
        assert!(ctx.is_idle());
    }

    #[test]
    fn test_is_idle_false() {
        let conn = Arc::new(MockConnection::new("conv-1", false));
        let ctx = ToolContext::new(conn);
        assert!(!ctx.is_idle());
    }

    #[tokio::test]
    async fn test_send_delegates_to_connection() {
        let conn = Arc::new(MockConnection::new("conv-1", false));
        let ctx = ToolContext::new(conn.clone());
        ctx.send("hello").await.unwrap();

        let messages = conn.sent_messages.lock().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], "hello");
    }

    #[tokio::test]
    async fn test_send_multiple_messages() {
        let conn = Arc::new(MockConnection::new("conv-1", false));
        let ctx = ToolContext::new(conn.clone());
        ctx.send("hello").await.unwrap();
        ctx.send("world").await.unwrap();

        let messages = conn.sent_messages.lock().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0], "hello");
        assert_eq!(messages[1], "world");
    }

    #[test]
    fn test_get_state_missing_returns_default() {
        let conn = Arc::new(MockConnection::new("conv-1", false));
        let ctx = ToolContext::new(conn);
        assert_eq!(ctx.get("missing"), None);
    }

    #[test]
    fn test_set_and_get_state() {
        let conn = Arc::new(MockConnection::new("conv-1", false));
        let mut ctx = ToolContext::new(conn);
        ctx.set("count".to_string(), Value::from(42));
        assert_eq!(ctx.get("count"), Some(&Value::from(42)));
    }

    #[test]
    fn test_set_state_overwrites() {
        let conn = Arc::new(MockConnection::new("conv-1", false));
        let mut ctx = ToolContext::new(conn);
        ctx.set("key".to_string(), Value::from("first"));
        ctx.set("key".to_string(), Value::from("second"));
        assert_eq!(ctx.get("key"), Some(&Value::from("second")));
    }

    #[test]
    fn test_state_isolation_between_instances() {
        let conn1 = Arc::new(MockConnection::new("conv-1", false));
        let conn2 = Arc::new(MockConnection::new("conv-2", false));
        let mut ctx1 = ToolContext::new(conn1);
        let ctx2 = ToolContext::new(conn2);
        ctx1.set("key".to_string(), Value::from(42));
        // ctx2 should not see ctx1's data
        assert_eq!(ctx2.get("key"), None);
    }
}
