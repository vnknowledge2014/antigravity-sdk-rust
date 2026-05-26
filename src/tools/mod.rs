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

//! Tool system for the Antigravity SDK.
//!
//! Manages registration and execution of custom tools.

pub mod tool_context;
pub mod tool_runner;

// Re-exports for ergonomic usage
pub use tool_context::ToolContext;
pub use tool_runner::{RegisteredTool, ToolHandler, ToolRunner, ToolWithSchema};
