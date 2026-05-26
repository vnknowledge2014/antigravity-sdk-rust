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

//! Trigger system for the Antigravity SDK.
//!
//! A Trigger is a long-lived async function that runs alongside an agent
//! session. It reacts to external events (cron, file changes, webhooks)
//! and pushes messages back into the agent.

pub mod helpers;
pub mod trigger_runner;
pub mod triggers;

// Re-exports for ergonomic usage
pub use helpers::{after, every};
pub use trigger_runner::TriggerRunner;
pub use triggers::{Trigger, TriggerContext};
