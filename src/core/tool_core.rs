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

//! Pure tool call processing logic.
//!
//! All functions are pure: they transform inputs into outputs without IO.

use std::collections::HashMap;

use crate::core::pipeline::Pipeline;
use crate::types::{ToolCall, ToolName, ToolResult};

/// Pure: Parse a wire-format tool call into a domain ToolCall.
///
/// Extracts the tool name, arguments JSON, and call ID from the
/// protobuf representation.
pub fn parse_wire_tool_call(
    id: Option<String>,
    name: Option<String>,
    arguments_json: Option<String>,
) -> Pipeline<ToolCall> {
    let tool_name = name.unwrap_or_default();
    let args: HashMap<String, serde_json::Value> = arguments_json
        .as_deref()
        .unwrap_or("{}")
        .parse::<serde_json::Value>()
        .ok()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    Ok(ToolCall {
        id,
        name: ToolName::Custom(tool_name),
        args,
        canonical_path: None,
    })
}

/// Pure: Build a denial ToolResult from a tool call and error message.
pub fn build_denial_result(tc: &ToolCall, message: &str) -> ToolResult {
    ToolResult {
        id: tc.id.clone(),
        name: tc.name.clone(),
        result: None,
        error: Some(format!("Tool execution denied by hook policy: {message}")),
    }
}

/// Pure: Resolve a tool result with potential error recovery.
///
/// If the result has an error and recovery is available, replaces
/// the error with the recovered value.
pub fn resolve_tool_result(
    mut result: ToolResult,
    error_recovery: Option<serde_json::Value>,
) -> ToolResult {
    if result.error.is_some() {
        if let Some(recovery) = error_recovery {
            result.error = None;
            result.result = Some(recovery);
        }
    }
    result
}

/// Pure: Build a ToolResult representing a missing tool runner error.
pub fn build_no_runner_result(tc: &ToolCall) -> ToolResult {
    ToolResult {
        id: tc.id.clone(),
        name: tc.name.clone(),
        result: None,
        error: Some("No tool runner configured to execute this tool.".into()),
    }
}

/// Pure: Check if a tool call deny message is meaningful.
pub fn effective_deny_message(message: &str) -> &str {
    if message.is_empty() {
        "No reason provided"
    } else {
        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wire_tool_call_full() {
        let result = parse_wire_tool_call(
            Some("tc-1".into()),
            Some("my_tool".into()),
            Some(r#"{"key":"value"}"#.into()),
        );
        let tc = result.unwrap();
        assert_eq!(tc.name.to_string(), "my_tool");
        assert_eq!(tc.id, Some("tc-1".into()));
        assert_eq!(tc.args.get("key").unwrap(), "value");
    }

    #[test]
    fn test_parse_wire_tool_call_empty() {
        let result = parse_wire_tool_call(None, None, None);
        let tc = result.unwrap();
        assert_eq!(tc.name.to_string(), "");
        assert!(tc.args.is_empty());
    }

    #[test]
    fn test_parse_wire_tool_call_invalid_json() {
        let result = parse_wire_tool_call(
            Some("tc-1".into()),
            Some("tool".into()),
            Some("not json".into()),
        );
        let tc = result.unwrap();
        assert!(tc.args.is_empty()); // Graceful fallback
    }

    #[test]
    fn test_build_denial_result() {
        let tc = ToolCall {
            name: ToolName::Custom("risky".into()),
            args: HashMap::new(),
            id: Some("tc-1".into()),
            canonical_path: None,
        };
        let result = build_denial_result(&tc, "policy denied");
        assert_eq!(result.id, Some("tc-1".into()));
        assert!(result.error.as_ref().unwrap().contains("policy denied"));
        assert!(result.result.is_none());
    }

    #[test]
    fn test_resolve_tool_result_no_error() {
        let result = ToolResult {
            name: ToolName::Custom("tool".into()),
            id: Some("tc-1".into()),
            result: Some(serde_json::json!("ok")),
            error: None,
        };
        let resolved = resolve_tool_result(result, Some(serde_json::json!("recovery")));
        // No error — recovery not applied
        assert_eq!(resolved.result, Some(serde_json::json!("ok")));
        assert!(resolved.error.is_none());
    }

    #[test]
    fn test_resolve_tool_result_with_recovery() {
        let result = ToolResult {
            name: ToolName::Custom("tool".into()),
            id: Some("tc-1".into()),
            result: None,
            error: Some("failed".into()),
        };
        let resolved = resolve_tool_result(result, Some(serde_json::json!("recovered")));
        assert!(resolved.error.is_none());
        assert_eq!(resolved.result, Some(serde_json::json!("recovered")));
    }

    #[test]
    fn test_resolve_tool_result_error_no_recovery() {
        let result = ToolResult {
            name: ToolName::Custom("tool".into()),
            id: Some("tc-1".into()),
            result: None,
            error: Some("failed".into()),
        };
        let resolved = resolve_tool_result(result, None);
        assert!(resolved.error.is_some());
    }

    #[test]
    fn test_build_no_runner_result() {
        let tc = ToolCall {
            name: ToolName::Custom("tool".into()),
            args: HashMap::new(),
            id: Some("tc-1".into()),
            canonical_path: None,
        };
        let result = build_no_runner_result(&tc);
        assert!(result.error.as_ref().unwrap().contains("No tool runner"));
    }

    #[test]
    fn test_effective_deny_message_empty() {
        assert_eq!(effective_deny_message(""), "No reason provided");
    }

    #[test]
    fn test_effective_deny_message_nonempty() {
        assert_eq!(effective_deny_message("blocked"), "blocked");
    }
}
