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

//! Functional Core — pure functions with zero side effects.
//!
//! This module follows the **Functional Core – Imperative Shell** pattern:
//! - All functions are pure (deterministic, no IO, no shared state)
//! - The imperative shell (Agent, LocalConnection) calls these functions
//!   and then executes the resulting IO operations
//!
//! Sub-modules:
//! - [`pipeline`] — Railway Oriented Programming types
//! - [`agent_core`] — Pure agent state transition logic
//! - [`tool_core`] — Pure tool call processing logic
//! - [`policy_core`] — Pure policy evaluation logic
//! - [`step_core`] — Pure step/history operations

pub mod pipeline;
pub mod agent_core;
pub mod tool_core;
pub mod policy_core;
pub mod step_core;
