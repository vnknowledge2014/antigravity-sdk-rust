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

use antigravity_sdk::mcp::McpBridge;
use antigravity_sdk::types::{
    McpServerConfig, McpSseServer, McpStdioServer, McpStreamableHttpServer,
};
use std::collections::HashMap;

#[tokio::test]
async fn test_connect_stdio() {
    let mut bridge = McpBridge::new();
    // Use an echoing dummy command or just check error since it's a real spawn
    let cmd = "echo";
    let args = vec!["{}".to_string()];

    // In our test environment, 'echo' will just immediately exit, so connect will likely fail or
    // finish immediately. We expect it to try to spawn and then return an error for initialization.
    // Testing the exact success without an actual MCP server is hard.
    let result = bridge.connect_stdio(cmd, &args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_connect_dispatch_stdio() {
    let mut bridge = McpBridge::new();
    let config = McpServerConfig::Stdio(McpStdioServer {
        command: "invalid_command_nonexistent_12345".to_string(),
        args: vec![],
    });

    let result = bridge.connect(&config).await;
    assert!(result.is_err());

    // Should fail with No such file or directory
    let err_str = result.unwrap_err().to_string();
    assert!(err_str.contains("No such file or directory") || err_str.contains("not found"));
}

#[tokio::test]
async fn test_connect_dispatch_sse() {
    let mut bridge = McpBridge::new();
    let config = McpServerConfig::Sse(McpSseServer {
        url: "http://invalid-url-that-does-not-exist:12345".to_string(),
        headers: None,
    });

    let result = bridge.connect(&config).await;
    // Expected to fail because of connection refused or DNS failure
    assert!(result.is_err());
}

#[tokio::test]
async fn test_connect_dispatch_http() {
    let mut bridge = McpBridge::new();
    let config = McpServerConfig::Http(McpStreamableHttpServer {
        url: "http://invalid-url-that-does-not-exist:12345".to_string(),
        headers: Some({
            let mut h = HashMap::new();
            h.insert("Authorization".to_string(), "Bearer test".to_string());
            h
        }),
        timeout: 10.0,
        sse_read_timeout: 300.0,
        terminate_on_close: true,
    });

    let result = bridge.connect(&config).await;
    // Expected to fail because of connection refused or DNS failure
    assert!(result.is_err());
}

#[tokio::test]
async fn test_bridge_lifecycle() {
    let mut bridge = McpBridge::new();
    // Stop shouldn't crash
    bridge.stop().await;
}

#[tokio::test]
async fn test_bridge_take_tools() {
    let mut bridge = McpBridge::new();
    let tools = bridge.take_tools();
    assert!(tools.is_empty());
}
