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

//! TriggerRunner — manages lifecycle of triggers.
//!
//! Corresponds to Python's `triggers/trigger_runner.py`.

use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::connections::Connection;
use crate::triggers::triggers::{Trigger, TriggerContext};

/// Manages registration, startup, and shutdown of triggers.
pub struct TriggerRunner {
    triggers: Vec<Trigger>,
    tasks: Vec<JoinHandle<()>>,
}

impl TriggerRunner {
    pub fn new(triggers: Vec<Trigger>) -> Self {
        Self {
            triggers,
            tasks: Vec::new(),
        }
    }

    /// Returns the number of registered triggers.
    pub fn trigger_count(&self) -> usize {
        self.triggers.len()
    }

    /// Start all triggers as concurrent tokio tasks.
    pub fn start(&mut self, connection: Arc<dyn Connection>) {
        for trigger in &self.triggers {
            let ctx = Arc::new(TriggerContext::new(connection.clone()));
            let trigger_future = trigger(ctx);
            let handle = tokio::spawn(async move {
                trigger_future.await;
            });
            self.tasks.push(handle);
        }
    }

    /// Cancel all trigger tasks.
    pub async fn stop(&mut self) {
        for task in self.tasks.drain(..) {
            task.abort();
            let _ = task.await;
        }
    }

    /// True if any trigger tasks are active.
    pub fn is_running(&self) -> bool {
        self.tasks.iter().any(|t| !t.is_finished())
    }

    /// Returns the number of active (non-finished) tasks.
    pub fn active_task_count(&self) -> usize {
        self.tasks.iter().filter(|t| !t.is_finished()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_runner_empty() {
        let runner = TriggerRunner::new(vec![]);
        assert_eq!(runner.trigger_count(), 0);
        assert!(!runner.is_running());
    }

    #[test]
    fn test_new_runner_with_triggers() {
        let triggers: Vec<Trigger> = vec![Box::new(|_ctx| Box::pin(async {}))];
        let runner = TriggerRunner::new(triggers);
        assert_eq!(runner.trigger_count(), 1);
    }

    #[tokio::test]
    async fn test_stop_empty_runner() {
        let mut runner = TriggerRunner::new(vec![]);
        runner.stop().await; // Should not panic
        assert!(!runner.is_running());
    }

    #[tokio::test]
    async fn test_active_task_count() {
        let runner = TriggerRunner::new(vec![]);
        assert_eq!(runner.active_task_count(), 0);
    }
}
