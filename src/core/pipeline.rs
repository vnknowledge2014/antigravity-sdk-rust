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

//! Railway Oriented Programming (ROP) pipeline types.
//!
//! Provides a unified error type and combinators for chaining async operations
//! along a "happy path" (Ok track) with automatic error short-circuiting
//! (Err track).
//!
//! # Design
//!
//! ```text
//! Ok  track: ──→ step1 ──→ step2 ──→ step3 ──→ Result
//!                  │          │          │
//! Err track: ─────→──────────→──────────→──────→ Error
//! ```
//!
//! Each step returns `Pipeline<T>`. If any step fails, subsequent steps
//! are skipped and the error propagates to the end.

use crate::types::{AntigravityConnectionError, ToolCall, ToolResult};
use std::fmt;

/// Unified error type for pipeline operations.
///
/// Categorizes all errors that can occur during a tool call or agent
/// lifecycle pipeline into semantic variants.
#[derive(Debug)]
pub enum PipelineError {
    /// A hook or policy denied the operation.
    Denied {
        message: String,
        tool_call: Option<ToolCall>,
    },

    /// A tool execution failed, possibly with error recovery.
    ToolError {
        name: String,
        error: String,
        recovery: Option<serde_json::Value>,
    },

    /// The underlying connection failed.
    ConnectionError(AntigravityConnectionError),

    /// A configuration or validation error.
    ValidationError(String),

    /// Any other internal error.
    Internal(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Denied { message, .. } => write!(f, "Denied: {message}"),
            Self::ToolError { name, error, .. } => write!(f, "Tool '{name}' error: {error}"),
            Self::ConnectionError(e) => write!(f, "Connection error: {e}"),
            Self::ValidationError(msg) => write!(f, "Validation error: {msg}"),
            Self::Internal(e) => write!(f, "Internal error: {e}"),
        }
    }
}

impl std::error::Error for PipelineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ConnectionError(e) => Some(e),
            Self::Internal(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<AntigravityConnectionError> for PipelineError {
    fn from(e: AntigravityConnectionError) -> Self {
        Self::ConnectionError(e)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for PipelineError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Internal(e)
    }
}

impl From<String> for PipelineError {
    fn from(s: String) -> Self {
        Self::ValidationError(s)
    }
}

/// Railway-oriented pipeline result type.
///
/// `Ok(T)` = happy path continues.
/// `Err(PipelineError)` = short-circuit to error handling.
pub type Pipeline<T> = Result<T, PipelineError>;

/// Intermediate state carried through the tool call pipeline.
#[derive(Debug)]
pub struct ToolPipelineState {
    pub tool_call: ToolCall,
    pub allowed: bool,
    pub deny_message: Option<String>,
    pub result: Option<ToolResult>,
}

impl ToolPipelineState {
    /// Creates a new pipeline state from a parsed tool call.
    pub fn new(tool_call: ToolCall) -> Self {
        Self {
            tool_call,
            allowed: true,
            deny_message: None,
            result: None,
        }
    }

    /// Marks this pipeline state as denied.
    pub fn deny(mut self, message: String) -> Self {
        self.allowed = false;
        self.deny_message = Some(message);
        self
    }

    /// Attaches a tool result.
    pub fn with_result(mut self, result: ToolResult) -> Self {
        self.result = Some(result);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolName;
    use std::collections::HashMap;

    fn make_test_tc() -> ToolCall {
        ToolCall {
            name: ToolName::Custom("test_tool".to_string()),
            args: HashMap::new(),
            id: Some("tc-1".to_string()),
            canonical_path: None,
        }
    }

    #[test]
    fn test_pipeline_ok_propagates() {
        let result: Pipeline<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_pipeline_err_short_circuits() {
        let result: Pipeline<i32> = Err(PipelineError::Denied {
            message: "blocked".into(),
            tool_call: None,
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("blocked"));
    }

    #[test]
    fn test_pipeline_chain_with_and_then() {
        let result: Pipeline<i32> = Ok(10)
            .and_then(|x| Ok(x * 2))
            .and_then(|x| Ok(x + 1));
        assert_eq!(result.unwrap(), 21);
    }

    #[test]
    fn test_pipeline_chain_short_circuit() {
        let result: Pipeline<i32> = Ok(10)
            .and_then(|_| Err(PipelineError::ValidationError("stop".into())))
            .and_then(|x: i32| Ok(x + 1)); // Never reached
        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_error_display() {
        let e = PipelineError::Denied {
            message: "policy".into(),
            tool_call: None,
        };
        assert_eq!(format!("{e}"), "Denied: policy");

        let e = PipelineError::ToolError {
            name: "run_cmd".into(),
            error: "fail".into(),
            recovery: None,
        };
        assert_eq!(format!("{e}"), "Tool 'run_cmd' error: fail");
    }

    #[test]
    fn test_pipeline_error_from_connection() {
        let conn_err = AntigravityConnectionError {
            message: "ws closed".into(),
        };
        let pe: PipelineError = conn_err.into();
        assert!(matches!(pe, PipelineError::ConnectionError(_)));
    }

    #[test]
    fn test_pipeline_error_from_string() {
        let pe: PipelineError = "invalid config".to_string().into();
        assert!(matches!(pe, PipelineError::ValidationError(_)));
    }

    #[test]
    fn test_tool_pipeline_state_new() {
        let state = ToolPipelineState::new(make_test_tc());
        assert!(state.allowed);
        assert!(state.deny_message.is_none());
        assert!(state.result.is_none());
    }

    #[test]
    fn test_tool_pipeline_state_deny() {
        let state = ToolPipelineState::new(make_test_tc()).deny("blocked".into());
        assert!(!state.allowed);
        assert_eq!(state.deny_message.as_deref(), Some("blocked"));
    }

    #[test]
    fn test_tool_pipeline_state_with_result() {
        let result = ToolResult {
            name: ToolName::Custom("test_tool".into()),
            id: Some("tc-1".into()),
            result: Some(serde_json::json!("ok")),
            error: None,
        };
        let state = ToolPipelineState::new(make_test_tc()).with_result(result);
        assert!(state.result.is_some());
    }

    #[test]
    fn test_pipeline_or_else_recovery() {
        let result: Pipeline<i32> = Err(PipelineError::ValidationError("fail".into()))
            .or_else(|_| Ok(99)); // Recover with default
        assert_eq!(result.unwrap(), 99);
    }

    #[test]
    fn test_pipeline_map_transforms_ok() {
        let result: Pipeline<String> = Ok(42).map(|x| format!("value={x}"));
        assert_eq!(result.unwrap(), "value=42");
    }

    #[test]
    fn test_pipeline_map_err_transforms_error() {
        let result: Pipeline<i32> = Err(PipelineError::ValidationError("x".into()))
            .map_err(|e| PipelineError::Internal(Box::new(std::io::Error::other(e.to_string()))));
        assert!(matches!(result.unwrap_err(), PipelineError::Internal(_)));
    }
}
