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

//! Google Antigravity SDK for building AI agents.
//!
//! The Rust-native implementation of the Antigravity SDK, following a
//! clean 3-layer architecture:
//!
//! - **Layer 1 (Agent)**: Entry point, lifecycle/config management.
//! - **Layer 2 (Conversation/Tools/Hooks)**: Stateful session management.
//! - **Layer 3 (Connection)**: Transport/backend abstraction.

// Enforce zero unsafe code at compile time.
#![forbid(unsafe_code)]

pub mod actors;
pub mod agent;
pub mod connections;
pub mod conversation;
pub mod core;
pub mod hooks;
pub mod mcp;
pub mod tools;
pub mod triggers;
pub mod types;
pub mod utils;

// Re-export primary public API
pub use agent::{Agent, AgentArgs};
pub use connections::{AgentConfig, Connection, ConnectionStrategy};
pub use conversation::Conversation;
pub use tools::ToolContext;
pub use types::{
    BuiltinTools, CapabilitiesConfig, GeminiConfig, GenerationConfig, ModelConfig, ModelEntry,
    ThinkingLevel, UsageMetadata,
};
