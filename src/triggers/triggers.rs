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

//! Trigger type definitions for the Antigravity SDK.
//!
//! Corresponds to Python's `triggers/triggers.py`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::connections::Connection;
use crate::types::AntigravityConnectionError;

/// Handle provided to every trigger at startup.
pub struct TriggerContext {
    connection: Arc<dyn Connection>,
}

impl TriggerContext {
    pub fn new(connection: Arc<dyn Connection>) -> Self {
        Self { connection }
    }

    /// Sends a message to the agent.
    pub async fn send(&self, content: &str) -> Result<(), AntigravityConnectionError> {
        self.connection.send_trigger_notification(content).await
    }
}

/// A Trigger is any async function that accepts a TriggerContext.
pub type Trigger =
    Box<dyn Fn(Arc<TriggerContext>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;

    // TriggerContext requires a connection, which is hard to mock without
    // a full mock object. These are basic structural tests.
    #[test]
    fn test_trigger_type_alias_compiles() {
        // Verify the type alias is valid
        let _: Option<Trigger> = None;
    }
}
