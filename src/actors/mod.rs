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

//! Actor-based concurrency model for the Antigravity SDK.
//!
//! Replaces the "Mutex Soup" pattern in `LocalConnection` with
//! message-passing actors that own their state exclusively.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────┐
//! │  WriterActor │ ← write_tx (bytes to WebSocket)
//! │  (WS Send)   │
//! └──────┬───────┘
//!        │ mpsc channel
//! ┌──────┴───────┐    ┌──────────────┐
//! │  StateActor  │───→│  StepStream  │
//! │  (Central)   │    │  (to Conv.)  │
//! └──────────────┘    └──────────────┘
//! ```
//!
//! **Key guarantee**: Each actor runs in exactly one `tokio::spawn` task,
//! owning all its state fields with zero `Mutex`/`RwLock` contention.

pub mod state_actor;
pub mod writer_actor;
