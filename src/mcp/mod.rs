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

//! MCP (Model Context Protocol) bridge for the Antigravity SDK.
//!
//! Simplifies the lifecycle of MCP Client Sessions and tool registration.

use std::collections::HashMap;
use std::sync::Arc;

use crate::tools::ToolWithSchema;
use crate::types::McpServerConfig;

use rmcp::model::{
    CallToolRequestParams, ClientCapabilities, Implementation, InitializeRequestParams,
};
use rmcp::serve_client;
use rmcp::service::{RoleClient, RunningService};
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::transport::streamable_http_client::{
    StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
};
use serde_json::Value;

type Session = Arc<RunningService<RoleClient, InitializeRequestParams>>;

/// Bridge between MCP services and the SDK ToolRunner.
pub struct McpBridge {
    tools: Vec<ToolWithSchema>,
    sessions: Vec<Session>,
}

impl Default for McpBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl McpBridge {
    /// Creates a new McpBridge.
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            sessions: Vec::new(),
        }
    }

    /// Takes the discovered tools out of the bridge.
    pub fn take_tools(&mut self) -> Vec<ToolWithSchema> {
        std::mem::take(&mut self.tools)
    }

    /// The MCP tools discovered from connected servers.
    pub fn tools(&self) -> &[ToolWithSchema] {
        &self.tools
    }

    /// Connects to an MCP server based on its configuration.
    pub async fn connect(
        &mut self,
        server_cfg: &McpServerConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match server_cfg {
            McpServerConfig::Stdio(cfg) => {
                self.connect_stdio(&cfg.command, &cfg.args).await?;
            }
            McpServerConfig::Sse(cfg) => {
                self.connect_sse(&cfg.url, cfg.headers.as_ref()).await?;
            }
            McpServerConfig::Http(cfg) => {
                self.connect_streamable_http(
                    &cfg.url,
                    cfg.headers.as_ref(),
                    cfg.timeout,
                    cfg.sse_read_timeout,
                    cfg.terminate_on_close,
                )
                .await?;
            }
        }
        Ok(())
    }

    fn client_info() -> InitializeRequestParams {
        InitializeRequestParams::new(
            ClientCapabilities::default(),
            Implementation::new("antigravity-rust-sdk", "0.1.0"),
        )
    }

    /// Connects to a local MCP server over stdio.
    pub async fn connect_stdio(
        &mut self,
        command: &str,
        args: &[String],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args);

        let process = TokioChildProcess::new(cmd)?;
        let session = serve_client(Self::client_info(), process).await?;
        self.add_session(Arc::new(session)).await?;
        Ok(())
    }

    /// Connects to a remote MCP server over SSE.
    pub async fn connect_sse(
        &mut self,
        url: &str,
        headers: Option<&HashMap<String, String>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut req_headers = HashMap::new();
        if let Some(h) = headers {
            for (k, v) in h {
                if let (Ok(name), Ok(val)) = (
                    reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                    reqwest::header::HeaderValue::from_str(v),
                ) {
                    req_headers.insert(name, val);
                }
            }
        }

        let mut config = StreamableHttpClientTransportConfig::default();
        config.uri = url.into();
        config.custom_headers = req_headers;
        let transport = StreamableHttpClientTransport::<reqwest::Client>::with_client(
            reqwest::Client::new(),
            config,
        );
        let session = serve_client(Self::client_info(), transport).await?;
        self.add_session(Arc::new(session)).await?;
        Ok(())
    }

    /// Connects to a remote MCP server over Streamable HTTP.
    pub async fn connect_streamable_http(
        &mut self,
        url: &str,
        headers: Option<&HashMap<String, String>>,
        _timeout: f64,
        _sse_read_timeout: f64,
        _terminate_on_close: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // In the rust SDK, streamable HTTP and SSE transports are functionally identical
        // using the rmcp StreamableHttpClient. Timeouts are managed by the underlying
        // reqwest client if configured, though rmcp defaults handle this internally.
        self.connect_sse(url, headers).await
    }

    /// Adds a session to the bridge, fetching and wrapping its tools.
    async fn add_session(
        &mut self,
        session: Session,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let tools_response = session.list_all_tools().await?;
        for tool in tools_response {
            let session_clone = Arc::clone(&session);
            let tool_name = tool.name.to_string();

            let description = tool
                .description
                .clone()
                .map(|d| d.to_string())
                .unwrap_or_default();
            let input_schema = tool.schema_as_json_value();

            let handler_name = tool_name.clone();
            let handler: crate::tools::ToolHandler = Box::new(move |args: Value| {
                let session = Arc::clone(&session_clone);
                let name_clone = handler_name.clone();
                Box::pin(async move {
                    let arguments = match args {
                        Value::Object(map) => Some(map),
                        _ => None,
                    };

                    let mut req = CallToolRequestParams::new(name_clone);
                    if let Some(args) = arguments {
                        req = req.with_arguments(args);
                    }

                    let result = session.call_tool(req).await?;
                    let val = serde_json::to_value(result)?;
                    Ok(val)
                })
            });

            self.tools.push(ToolWithSchema {
                name: tool_name,
                description,
                input_schema,
                handler,
            });
        }

        self.sessions.push(session);
        Ok(())
    }

    /// Cleans up all active MCP sessions.
    pub async fn stop(&mut self) {
        // `RunningService` automatically stops when dropped, but we can explicitly cancel.
        // Wait, rmcp provides cancellation, but simply dropping the Arc or clearing
        // `self.sessions` effectively kills the client processes and tasks.
        self.sessions.clear();
        self.tools.clear();
    }
}
