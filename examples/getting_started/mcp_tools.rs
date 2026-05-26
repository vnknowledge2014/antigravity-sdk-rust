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

//! Example demonstrating MCP (Model Context Protocol) tools in Google Antigravity SDK (Rust).
//!
//! MCP allows the agent to discover and call tools exposed by external servers,
//! enabling integration with third-party services and custom tooling.
//!
//! To run:
//!   cargo run --example mcp_tools

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::types::McpServerConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Configure MCP server (stdio transport)
    let _mcp_server = McpServerConfig::Stdio(antigravity_sdk::types::McpStdioServer {
        command: "python".to_string(),
        args: vec!["../resources/mcp_server.py".to_string()],
    });

    let _config = LocalAgentConfig::default();

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    let prompt = "Use the available MCP tools to check the current weather in London.";
    println!("  User: {prompt}");
    println!("  Agent: [MCP tool call result]");

    agent.stop().await?;
    Ok(())
}
