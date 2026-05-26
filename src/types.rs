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

//! Type definitions for the Google Antigravity SDK.
//!
//! These are the canonical SDK boundary types. All public SDK interfaces use
//! these types. They are pure Rust structs with `serde` serialization — no
//! proto dependencies at this layer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

// =============================================================================
// Error types
// =============================================================================

/// Base error type for connection-level failures.
#[derive(Debug, Error)]
#[error("{message}")]
pub struct AntigravityConnectionError {
    pub message: String,
}

/// Wraps validation errors at the SDK boundary.
#[derive(Debug, Error)]
#[error("{message}")]
pub struct AntigravityValidationError {
    pub message: String,
    pub errors: Vec<serde_json::Value>,
}

// =============================================================================
// Config types
// =============================================================================

pub const DEFAULT_MODEL: &str = "gemini-3.5-flash";
pub const DEFAULT_IMAGE_GENERATION_MODEL: &str = "gemini-3.1-flash-image-preview";

/// Thinking level for Gemini models that support extended thinking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    Minimal,
    Low,
    Medium,
    High,
}

/// Generation parameters for a model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenerationConfig {
    pub thinking_level: Option<ThinkingLevel>,
}

/// A model with optional auth and generation overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub name: String,
    pub api_key: Option<String>,
    #[serde(default)]
    pub generation: GenerationConfig,
}

impl Default for ModelEntry {
    fn default() -> Self {
        Self {
            name: DEFAULT_MODEL.to_string(),
            api_key: None,
            generation: GenerationConfig::default(),
        }
    }
}

impl From<&str> for ModelEntry {
    fn from(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }
}

impl From<String> for ModelEntry {
    fn from(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }
}

/// Model selection for each capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    #[serde(default)]
    pub default: ModelEntry,
    #[serde(default = "default_image_model")]
    pub image_generation: ModelEntry,
}

fn default_image_model() -> ModelEntry {
    ModelEntry {
        name: DEFAULT_IMAGE_GENERATION_MODEL.to_string(),
        ..Default::default()
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default: ModelEntry::default(),
            image_generation: default_image_model(),
        }
    }
}

/// Configuration for the Gemini model backend.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiConfig {
    pub api_key: Option<String>,
    #[serde(default)]
    pub models: ModelConfig,
}

/// Configuration for OpenRouter, Ollama, and OpenAI-compatible endpoints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GemmaConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model_name: Option<String>,
}

/// Configuration for the Anthropic API backend.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnthropicConfig {
    pub api_key: Option<String>,
    pub model_name: Option<String>,
    pub thinking_level: Option<String>,
}

/// Enum for selecting the backend provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider")]
pub enum ModelBackend {
    Gemini(GeminiConfig),
    OpenAICompatible(GemmaConfig),
    Anthropic(AnthropicConfig),
}

impl Default for ModelBackend {
    fn default() -> Self {
        Self::Gemini(GeminiConfig::default())
    }
}


/// A named section to append to the system instructions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInstructionSection {
    pub content: String,
    #[serde(default = "default_section_title")]
    pub title: String,
}

fn default_section_title() -> String {
    "user_system_instructions".to_string()
}

/// Completely replace the system instructions (advanced usage).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSystemInstructions {
    pub text: String,
}

/// Append sections to the default system instructions (recommended).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplatedSystemInstructions {
    pub identity: Option<String>,
    #[serde(default)]
    pub sections: Vec<SystemInstructionSection>,
}

/// Union type for system instructions configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SystemInstructions {
    Custom(CustomSystemInstructions),
    Templated(TemplatedSystemInstructions),
}

// =============================================================================
// Builtin Tools
// =============================================================================

/// Identifiers for common connection-provided builtin tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuiltinTools {
    #[serde(rename = "list_directory")]
    ListDir,
    #[serde(rename = "search_directory")]
    SearchDir,
    #[serde(rename = "find_file")]
    FindFile,
    #[serde(rename = "view_file")]
    ViewFile,
    #[serde(rename = "create_file")]
    CreateFile,
    #[serde(rename = "edit_file")]
    EditFile,
    #[serde(rename = "run_command")]
    RunCommand,
    #[serde(rename = "ask_question")]
    AskQuestion,
    #[serde(rename = "start_subagent")]
    StartSubagent,
    #[serde(rename = "generate_image")]
    GenerateImage,
    #[serde(rename = "finish")]
    Finish,
}

impl BuiltinTools {
    /// Returns the string value of this tool (matching the Python SDK).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ListDir => "list_directory",
            Self::SearchDir => "search_directory",
            Self::FindFile => "find_file",
            Self::ViewFile => "view_file",
            Self::CreateFile => "create_file",
            Self::EditFile => "edit_file",
            Self::RunCommand => "run_command",
            Self::AskQuestion => "ask_question",
            Self::StartSubagent => "start_subagent",
            Self::GenerateImage => "generate_image",
            Self::Finish => "finish",
        }
    }

    /// Tools that only read state (no writes, deletes, or commands).
    pub fn read_only() -> Vec<BuiltinTools> {
        vec![
            Self::ListDir,
            Self::SearchDir,
            Self::FindFile,
            Self::ViewFile,
            Self::Finish,
        ]
    }

    /// Tools that cannot delete content.
    pub fn nondestructive() -> Vec<BuiltinTools> {
        vec![
            Self::ListDir,
            Self::SearchDir,
            Self::FindFile,
            Self::ViewFile,
            Self::CreateFile,
            Self::EditFile,
            Self::AskQuestion,
            Self::StartSubagent,
            Self::GenerateImage,
            Self::Finish,
        ]
    }

    /// All builtin tools.
    pub fn all_tools() -> Vec<BuiltinTools> {
        vec![
            Self::ListDir,
            Self::SearchDir,
            Self::FindFile,
            Self::ViewFile,
            Self::CreateFile,
            Self::EditFile,
            Self::RunCommand,
            Self::AskQuestion,
            Self::StartSubagent,
            Self::GenerateImage,
            Self::Finish,
        ]
    }

    /// Tools that perform file read/write/create operations.
    pub fn file_tools() -> Vec<BuiltinTools> {
        vec![Self::ViewFile, Self::CreateFile, Self::EditFile]
    }
}

// =============================================================================
// Capabilities Config
// =============================================================================

/// General agent capability configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesConfig {
    #[serde(default = "default_true")]
    pub enable_subagents: bool,
    pub enabled_tools: Option<Vec<BuiltinTools>>,
    pub disabled_tools: Option<Vec<BuiltinTools>>,
    pub compaction_threshold: Option<u32>,
    #[serde(default = "default_image_model_name")]
    pub image_model: String,
    pub finish_tool_schema_json: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_image_model_name() -> String {
    DEFAULT_IMAGE_GENERATION_MODEL.to_string()
}

impl Default for CapabilitiesConfig {
    fn default() -> Self {
        Self {
            enable_subagents: true,
            enabled_tools: None,
            disabled_tools: None,
            compaction_threshold: None,
            image_model: DEFAULT_IMAGE_GENERATION_MODEL.to_string(),
            finish_tool_schema_json: None,
        }
    }
}

impl CapabilitiesConfig {
    /// Validates that enabled_tools and disabled_tools are mutually exclusive.
    pub fn validate(&self) -> Result<(), AntigravityValidationError> {
        if self.enabled_tools.is_some() && self.disabled_tools.is_some() {
            return Err(AntigravityValidationError {
                message: "enabled_tools and disabled_tools should be mutually exclusive."
                    .to_string(),
                errors: vec![],
            });
        }
        Ok(())
    }
}

// =============================================================================
// MCP Server Configs
// =============================================================================

/// Configuration for an MCP server connected via stdio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStdioServer {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// Configuration for an MCP server connected via SSE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSseServer {
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
}

/// Configuration for an MCP server connected via Streamable HTTP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStreamableHttpServer {
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
    #[serde(default = "default_timeout")]
    pub timeout: f64,
    #[serde(default = "default_sse_read_timeout")]
    pub sse_read_timeout: f64,
    #[serde(default = "default_true")]
    pub terminate_on_close: bool,
}

fn default_timeout() -> f64 {
    30.0
}

fn default_sse_read_timeout() -> f64 {
    300.0
}

/// Union type for MCP server configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpServerConfig {
    #[serde(rename = "stdio")]
    Stdio(McpStdioServer),
    #[serde(rename = "sse")]
    Sse(McpSseServer),
    #[serde(rename = "http")]
    Http(McpStreamableHttpServer),
}

// =============================================================================
// Tool types
// =============================================================================

/// The name of a tool — either a builtin or a custom string name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolName {
    Builtin(BuiltinTools),
    Custom(String),
}

impl From<&str> for ToolName {
    fn from(s: &str) -> Self {
        // Try to parse as builtin first
        if let Ok(builtin) = serde_json::from_value(serde_json::Value::String(s.to_string())) {
            ToolName::Builtin(builtin)
        } else {
            ToolName::Custom(s.to_string())
        }
    }
}

impl std::fmt::Display for ToolName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolName::Builtin(b) => write!(f, "{}", b.as_str()),
            ToolName::Custom(s) => write!(f, "{s}"),
        }
    }
}

/// A tool call to inject into the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: ToolName,
    #[serde(default)]
    pub args: HashMap<String, serde_json::Value>,
    pub id: Option<String>,
    pub canonical_path: Option<String>,
}

/// Result of a single tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub name: ToolName,
    pub id: Option<String>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

// =============================================================================
// Step types
// =============================================================================

/// Token usage metadata from the model API.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageMetadata {
    pub prompt_token_count: Option<i32>,
    pub cached_content_token_count: Option<i32>,
    pub candidates_token_count: Option<i32>,
    pub thoughts_token_count: Option<i32>,
    pub total_token_count: Option<i32>,
}

/// High-level type of a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Default)]
pub enum StepType {
    TextResponse,
    ToolCall,
    SystemMessage,
    Compaction,
    Finish,
    #[default]
    Unknown,
}

/// Source of a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Default)]
pub enum StepSource {
    System,
    User,
    Model,
    #[default]
    Unknown,
}

/// Target of a step interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StepTarget {
    #[serde(rename = "TARGET_USER")]
    User,
    #[serde(rename = "TARGET_ENVIRONMENT")]
    Environment,
    #[serde(rename = "TARGET_UNSPECIFIED")]
    Unspecified,
    #[default]
    Unknown,
}

/// Status of a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Default)]
pub enum StepStatus {
    Active,
    Done,
    WaitingForUser,
    Error,
    Canceled,
    #[default]
    Unknown,
}

/// Structure representing one action in the agent trajectory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Step {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub step_index: u32,
    #[serde(default)]
    pub r#type: StepType,
    #[serde(default)]
    pub source: StepSource,
    #[serde(default)]
    pub target: StepTarget,
    #[serde(default)]
    pub status: StepStatus,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub content_delta: String,
    #[serde(default)]
    pub thinking: String,
    #[serde(default)]
    pub thinking_delta: String,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default)]
    pub error: String,
    pub is_complete_response: Option<bool>,
    pub structured_output: Option<serde_json::Value>,
    pub usage_metadata: Option<UsageMetadata>,
}

// =============================================================================
// Hook types
// =============================================================================

/// Result of a decision hook execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    #[serde(default = "default_true")]
    pub allow: bool,
    #[serde(default)]
    pub message: String,
}

impl Default for HookResult {
    fn default() -> Self {
        Self {
            allow: true,
            message: String::new(),
        }
    }
}

impl HookResult {
    pub fn allowed() -> Self {
        Self {
            allow: true,
            message: String::new(),
        }
    }

    pub fn denied(message: impl Into<String>) -> Self {
        Self {
            allow: false,
            message: message.into(),
        }
    }
}

// =============================================================================
// Question / Interaction types
// =============================================================================

/// Individual response for an AskQuestion entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuestionResponse {
    pub selected_option_ids: Option<Vec<String>>,
    #[serde(default)]
    pub freeform_response: String,
    #[serde(default)]
    pub skipped: bool,
}

/// Result of an interaction containing a list of responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionHookResult {
    pub responses: Vec<QuestionResponse>,
    #[serde(default)]
    pub cancelled: bool,
}

/// Option for an AskQuestion entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskQuestionOption {
    pub id: String,
    pub text: String,
}

/// A single question with predefined options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskQuestionEntry {
    pub question: String,
    pub options: Vec<AskQuestionOption>,
    #[serde(default)]
    pub is_multi_select: bool,
}

/// Interaction spec for ask_question dialog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskQuestionInteractionSpec {
    pub questions: Vec<AskQuestionEntry>,
}

// =============================================================================
// Trigger types
// =============================================================================

/// Controls how trigger messages are delivered to the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerDelivery {
    SendImmediately,
    WaitIdle,
}

/// Kind of filesystem change detected by a file-watching trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileChangeKind {
    Added,
    Modified,
    Deleted,
}

/// A single filesystem change detected by a file-watching trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub kind: FileChangeKind,
    pub path: String,
}

// =============================================================================
// Streaming response types
// =============================================================================

/// A delta chunk representing a piece of the model's internal reasoning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    pub step_index: u32,
    pub text: String,
    pub signature: Option<Vec<u8>>,
}

/// A delta chunk representing a piece of the model's text output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
    pub step_index: u32,
    pub text: String,
}

/// Semantic chunks yielded during streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamChunk {
    Thought(Thought),
    Text(Text),
    ToolCall(ToolCall),
    ToolResult(ToolResult),
}

// =============================================================================
// Content primitives
// =============================================================================

/// Supported image MIME types.
pub const SUPPORTED_IMAGE_MIMES: &[&str] = &["image/bmp", "image/jpeg", "image/png", "image/webp"];

/// Supported document MIME types.
pub const SUPPORTED_DOCUMENT_MIMES: &[&str] = &[
    "application/pdf",
    "application/json",
    "text/css",
    "text/csv",
    "text/html",
    "text/javascript",
    "text/plain",
    "text/rtf",
    "text/xml",
];

/// Guesses MIME type from file extension (covers SDK-supported types).
fn guess_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("bmp") => "image/bmp",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("pdf") => "application/pdf",
        Some("json") => "application/json",
        Some("css") => "text/css",
        Some("csv") => "text/csv",
        Some("html" | "htm") => "text/html",
        Some("js") => "text/javascript",
        Some("txt") => "text/plain",
        Some("rtf") => "text/rtf",
        Some("xml") => "text/xml",
        Some("wav") => "audio/wav",
        Some("mp3") => "audio/mp3",
        Some("aac") => "audio/aac",
        Some("ogg") => "audio/ogg",
        Some("flac") => "audio/flac",
        Some("opus") => "audio/opus",
        Some("mp4") => "video/mp4",
        Some("mpeg" | "mpg") => "video/mpeg",
        Some("avi") => "video/avi",
        Some("webm") => "video/webm",
        Some("mov") => "video/quicktime",
        _ => "application/octet-stream",
    }
}

/// Base media content attachment.
#[derive(Debug, Clone)]
pub struct Media {
    pub data: Vec<u8>,
    pub mime_type: String,
    pub description: Option<String>,
}

impl Media {
    /// Load media from a local file path.
    pub fn from_file(path: impl AsRef<Path>, description: Option<String>) -> std::io::Result<Self> {
        let path = path.as_ref();
        let data = std::fs::read(path)?;
        let mime_type = guess_mime_type(path).to_string();
        Ok(Self {
            data,
            mime_type,
            description,
        })
    }
}

/// A prompt content primitive — text or media.
#[derive(Debug, Clone)]
pub enum ContentPrimitive {
    Text(String),
    Media(Media),
}

impl From<&str> for ContentPrimitive {
    fn from(s: &str) -> Self {
        ContentPrimitive::Text(s.to_string())
    }
}

impl From<String> for ContentPrimitive {
    fn from(s: String) -> Self {
        ContentPrimitive::Text(s)
    }
}

/// Content that can be sent to the agent.
#[derive(Debug, Clone)]
pub enum Content {
    Single(ContentPrimitive),
    Multiple(Vec<ContentPrimitive>),
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Content::Single(ContentPrimitive::Text(s.to_string()))
    }
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Content::Single(ContentPrimitive::Text(s))
    }
}

// =============================================================================
// ChatResponse
// =============================================================================

use futures::Stream;
use std::pin::Pin;

pub struct ChatResponse<'a> {
    pub chunk_stream: Pin<Box<dyn Stream<Item = StreamChunk> + Send + 'a>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Error types
    // =========================================================================

    #[test]
    fn test_connection_error_display() {
        let err = AntigravityConnectionError {
            message: "connection refused".to_string(),
        };
        assert_eq!(format!("{err}"), "connection refused");
    }

    #[test]
    fn test_validation_error_display() {
        let err = AntigravityValidationError {
            message: "invalid config".to_string(),
            errors: vec![serde_json::json!({"field": "model"})],
        };
        assert_eq!(format!("{err}"), "invalid config");
        assert_eq!(err.errors.len(), 1);
    }

    // =========================================================================
    // Config types
    // =========================================================================

    #[test]
    fn test_default_model_constant() {
        assert_eq!(DEFAULT_MODEL, "gemini-3.5-flash");
    }

    #[test]
    fn test_default_image_model_constant() {
        assert_eq!(
            DEFAULT_IMAGE_GENERATION_MODEL,
            "gemini-3.1-flash-image-preview"
        );
    }

    #[test]
    fn test_thinking_level_serde_roundtrip() {
        let level = ThinkingLevel::High;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"high\"");
        let parsed: ThinkingLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ThinkingLevel::High);
    }

    #[test]
    fn test_thinking_level_all_variants() {
        for (variant, expected) in [
            (ThinkingLevel::Minimal, "\"minimal\""),
            (ThinkingLevel::Low, "\"low\""),
            (ThinkingLevel::Medium, "\"medium\""),
            (ThinkingLevel::High, "\"high\""),
        ] {
            assert_eq!(serde_json::to_string(&variant).unwrap(), expected);
        }
    }

    #[test]
    fn test_generation_config_default() {
        let gc = GenerationConfig::default();
        assert!(gc.thinking_level.is_none());
    }

    #[test]
    fn test_model_entry_default() {
        let entry = ModelEntry::default();
        assert_eq!(entry.name, DEFAULT_MODEL);
        assert!(entry.api_key.is_none());
    }

    #[test]
    fn test_model_entry_from_str() {
        let entry = ModelEntry::from("gemini-2.0-pro");
        assert_eq!(entry.name, "gemini-2.0-pro");
        assert!(entry.api_key.is_none());
    }

    #[test]
    fn test_model_entry_from_string() {
        let entry = ModelEntry::from("custom-model".to_string());
        assert_eq!(entry.name, "custom-model");
    }

    #[test]
    fn test_model_config_default() {
        let mc = ModelConfig::default();
        assert_eq!(mc.default.name, DEFAULT_MODEL);
        assert_eq!(mc.image_generation.name, DEFAULT_IMAGE_GENERATION_MODEL);
    }

    #[test]
    fn test_gemini_config_default() {
        let gc = GeminiConfig::default();
        assert!(gc.api_key.is_none());
        assert_eq!(gc.models.default.name, DEFAULT_MODEL);
    }

    #[test]
    fn test_system_instruction_section() {
        let section = SystemInstructionSection {
            content: "Be helpful".to_string(),
            title: "persona".to_string(),
        };
        assert_eq!(section.title, "persona");
        assert_eq!(section.content, "Be helpful");
    }

    // =========================================================================
    // Builtin Tools
    // =========================================================================

    #[test]
    fn test_builtin_tools_as_str() {
        assert_eq!(BuiltinTools::ListDir.as_str(), "list_directory");
        assert_eq!(BuiltinTools::RunCommand.as_str(), "run_command");
        assert_eq!(BuiltinTools::AskQuestion.as_str(), "ask_question");
        assert_eq!(BuiltinTools::Finish.as_str(), "finish");
    }

    #[test]
    fn test_builtin_tools_serde_roundtrip() {
        let tool = BuiltinTools::ViewFile;
        let json = serde_json::to_string(&tool).unwrap();
        assert_eq!(json, "\"view_file\"");
        let parsed: BuiltinTools = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, BuiltinTools::ViewFile);
    }

    #[test]
    fn test_builtin_tools_read_only() {
        let ro = BuiltinTools::read_only();
        assert!(ro.contains(&BuiltinTools::ViewFile));
        assert!(ro.contains(&BuiltinTools::Finish));
        assert!(!ro.contains(&BuiltinTools::RunCommand));
        assert!(!ro.contains(&BuiltinTools::EditFile));
    }

    #[test]
    fn test_builtin_tools_nondestructive() {
        let nd = BuiltinTools::nondestructive();
        assert!(nd.contains(&BuiltinTools::ViewFile));
        assert!(nd.contains(&BuiltinTools::CreateFile));
        assert!(nd.contains(&BuiltinTools::EditFile));
        assert!(!nd.contains(&BuiltinTools::RunCommand));
    }

    #[test]
    fn test_builtin_tools_all_tools() {
        let all = BuiltinTools::all_tools();
        assert_eq!(all.len(), 11); // All variants
        assert!(all.contains(&BuiltinTools::RunCommand));
    }

    #[test]
    fn test_builtin_tools_file_tools() {
        let ft = BuiltinTools::file_tools();
        assert_eq!(ft.len(), 3);
        assert!(ft.contains(&BuiltinTools::ViewFile));
        assert!(ft.contains(&BuiltinTools::CreateFile));
        assert!(ft.contains(&BuiltinTools::EditFile));
    }

    // =========================================================================
    // Capabilities Config
    // =========================================================================

    #[test]
    fn test_capabilities_config_default() {
        let cc = CapabilitiesConfig::default();
        assert!(cc.enable_subagents);
        assert!(cc.enabled_tools.is_none());
        assert!(cc.disabled_tools.is_none());
        assert!(cc.compaction_threshold.is_none());
    }

    #[test]
    fn test_capabilities_config_validate_ok() {
        let cc = CapabilitiesConfig::default();
        assert!(cc.validate().is_ok());
    }

    #[test]
    fn test_capabilities_config_validate_mutually_exclusive() {
        let cc = CapabilitiesConfig {
            enabled_tools: Some(vec![BuiltinTools::ViewFile]),
            disabled_tools: Some(vec![BuiltinTools::RunCommand]),
            ..Default::default()
        };
        let result = cc.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("mutually exclusive"));
    }

    #[test]
    fn test_capabilities_config_enabled_tools_only() {
        let cc = CapabilitiesConfig {
            enabled_tools: Some(vec![BuiltinTools::ViewFile]),
            ..Default::default()
        };
        assert!(cc.validate().is_ok());
    }

    #[test]
    fn test_capabilities_config_disabled_tools_only() {
        let cc = CapabilitiesConfig {
            disabled_tools: Some(vec![BuiltinTools::RunCommand]),
            ..Default::default()
        };
        assert!(cc.validate().is_ok());
    }

    // =========================================================================
    // MCP Server Config
    // =========================================================================

    #[test]
    fn test_mcp_stdio_server() {
        let json = r#"{"type":"stdio","command":"npx","args":["server"]}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        match config {
            McpServerConfig::Stdio(s) => {
                assert_eq!(s.command, "npx");
                assert_eq!(s.args, vec!["server"]);
            }
            _ => panic!("Expected Stdio variant"),
        }
    }

    #[test]
    fn test_mcp_sse_server() {
        let json = r#"{"type":"sse","url":"http://localhost:8080"}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        match config {
            McpServerConfig::Sse(s) => {
                assert_eq!(s.url, "http://localhost:8080");
                assert!(s.headers.is_none());
            }
            _ => panic!("Expected Sse variant"),
        }
    }

    #[test]
    fn test_mcp_http_server_defaults() {
        let json = r#"{"type":"http","url":"http://localhost:9090"}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        match config {
            McpServerConfig::Http(s) => {
                assert_eq!(s.url, "http://localhost:9090");
                assert_eq!(s.timeout, 30.0);
                assert_eq!(s.sse_read_timeout, 300.0);
                assert!(s.terminate_on_close);
            }
            _ => panic!("Expected Http variant"),
        }
    }

    // =========================================================================
    // Tool types
    // =========================================================================

    #[test]
    fn test_tool_name_from_builtin_str() {
        let name = ToolName::from("view_file");
        assert_eq!(name, ToolName::Builtin(BuiltinTools::ViewFile));
    }

    #[test]
    fn test_tool_name_from_custom_str() {
        let name = ToolName::from("my_custom_tool");
        assert_eq!(name, ToolName::Custom("my_custom_tool".to_string()));
    }

    #[test]
    fn test_tool_name_display_builtin() {
        let name = ToolName::Builtin(BuiltinTools::RunCommand);
        assert_eq!(format!("{name}"), "run_command");
    }

    #[test]
    fn test_tool_name_display_custom() {
        let name = ToolName::Custom("my_tool".to_string());
        assert_eq!(format!("{name}"), "my_tool");
    }

    #[test]
    fn test_tool_call_serde() {
        let tc = ToolCall {
            name: ToolName::Custom("echo".to_string()),
            args: {
                let mut m = HashMap::new();
                m.insert("msg".to_string(), serde_json::json!("hello"));
                m
            },
            id: Some("call-1".to_string()),
            canonical_path: None,
        };
        let json = serde_json::to_string(&tc).unwrap();
        let parsed: ToolCall = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, Some("call-1".to_string()));
    }

    #[test]
    fn test_tool_result_success() {
        let tr = ToolResult {
            name: ToolName::Custom("echo".to_string()),
            id: Some("call-1".to_string()),
            result: Some(serde_json::json!({"output": "hello"})),
            error: None,
        };
        assert!(tr.error.is_none());
        assert!(tr.result.is_some());
    }

    #[test]
    fn test_tool_result_error() {
        let tr = ToolResult {
            name: ToolName::Custom("fail".to_string()),
            id: Some("call-2".to_string()),
            result: None,
            error: Some("tool crashed".to_string()),
        };
        assert!(tr.result.is_none());
        assert_eq!(tr.error.as_deref(), Some("tool crashed"));
    }

    // =========================================================================
    // Step types
    // =========================================================================

    #[test]
    fn test_step_type_serde() {
        let st = StepType::TextResponse;
        let json = serde_json::to_string(&st).unwrap();
        assert_eq!(json, "\"TEXT_RESPONSE\"");
        let parsed: StepType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, StepType::TextResponse);
    }

    #[test]
    fn test_step_type_all_variants() {
        for (variant, expected) in [
            (StepType::TextResponse, "\"TEXT_RESPONSE\""),
            (StepType::ToolCall, "\"TOOL_CALL\""),
            (StepType::SystemMessage, "\"SYSTEM_MESSAGE\""),
            (StepType::Compaction, "\"COMPACTION\""),
            (StepType::Finish, "\"FINISH\""),
            (StepType::Unknown, "\"UNKNOWN\""),
        ] {
            assert_eq!(serde_json::to_string(&variant).unwrap(), expected);
        }
    }

    #[test]
    fn test_step_source_default() {
        assert_eq!(StepSource::default(), StepSource::Unknown);
    }

    #[test]
    fn test_step_target_serde() {
        let target = StepTarget::User;
        let json = serde_json::to_string(&target).unwrap();
        assert_eq!(json, "\"TARGET_USER\"");
    }

    #[test]
    fn test_step_status_all_variants() {
        for (variant, expected) in [
            (StepStatus::Active, "\"ACTIVE\""),
            (StepStatus::Done, "\"DONE\""),
            (StepStatus::WaitingForUser, "\"WAITING_FOR_USER\""),
            (StepStatus::Error, "\"ERROR\""),
            (StepStatus::Canceled, "\"CANCELED\""),
            (StepStatus::Unknown, "\"UNKNOWN\""),
        ] {
            assert_eq!(serde_json::to_string(&variant).unwrap(), expected);
        }
    }

    #[test]
    fn test_step_default() {
        let step = Step::default();
        assert_eq!(step.r#type, StepType::Unknown);
        assert_eq!(step.source, StepSource::Unknown);
        assert_eq!(step.status, StepStatus::Unknown);
        assert!(step.content.is_empty());
        assert!(step.tool_calls.is_empty());
    }

    #[test]
    fn test_step_serde_roundtrip() {
        let step = Step {
            id: "step-1".to_string(),
            step_index: 0,
            r#type: StepType::TextResponse,
            source: StepSource::Model,
            target: StepTarget::User,
            status: StepStatus::Done,
            content: "Hello world".to_string(),
            content_delta: String::new(),
            thinking: String::new(),
            thinking_delta: String::new(),
            tool_calls: vec![],
            error: String::new(),
            is_complete_response: Some(true),
            structured_output: None,
            usage_metadata: None,
        };
        let json = serde_json::to_string(&step).unwrap();
        let parsed: Step = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "step-1");
        assert_eq!(parsed.content, "Hello world");
        assert_eq!(parsed.r#type, StepType::TextResponse);
    }

    #[test]
    fn test_usage_metadata_default() {
        let um = UsageMetadata::default();
        assert!(um.prompt_token_count.is_none());
        assert!(um.total_token_count.is_none());
    }

    #[test]
    fn test_usage_metadata_serde() {
        let um = UsageMetadata {
            prompt_token_count: Some(100),
            cached_content_token_count: None,
            candidates_token_count: Some(50),
            thoughts_token_count: Some(10),
            total_token_count: Some(160),
        };
        let json = serde_json::to_string(&um).unwrap();
        let parsed: UsageMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.prompt_token_count, Some(100));
        assert_eq!(parsed.total_token_count, Some(160));
    }

    // =========================================================================
    // Hook types
    // =========================================================================

    #[test]
    fn test_hook_result_default() {
        let hr = HookResult::default();
        assert!(hr.allow);
        assert!(hr.message.is_empty());
    }

    #[test]
    fn test_hook_result_allowed() {
        let hr = HookResult::allowed();
        assert!(hr.allow);
    }

    #[test]
    fn test_hook_result_denied() {
        let hr = HookResult::denied("blocked by policy");
        assert!(!hr.allow);
        assert_eq!(hr.message, "blocked by policy");
    }

    #[test]
    fn test_hook_result_denied_from_string() {
        let hr = HookResult::denied("test".to_string());
        assert!(!hr.allow);
    }

    // =========================================================================
    // Question / Interaction types
    // =========================================================================

    #[test]
    fn test_question_response_default() {
        let qr = QuestionResponse::default();
        assert!(qr.selected_option_ids.is_none());
        assert!(qr.freeform_response.is_empty());
        assert!(!qr.skipped);
    }

    #[test]
    fn test_ask_question_entry() {
        let entry = AskQuestionEntry {
            question: "Continue?".to_string(),
            options: vec![
                AskQuestionOption {
                    id: "y".to_string(),
                    text: "Yes".to_string(),
                },
                AskQuestionOption {
                    id: "n".to_string(),
                    text: "No".to_string(),
                },
            ],
            is_multi_select: false,
        };
        assert_eq!(entry.options.len(), 2);
        assert!(!entry.is_multi_select);
    }

    #[test]
    fn test_ask_question_interaction_spec_serde() {
        let spec = AskQuestionInteractionSpec {
            questions: vec![AskQuestionEntry {
                question: "Q1".to_string(),
                options: vec![],
                is_multi_select: false,
            }],
        };
        let json = serde_json::to_string(&spec).unwrap();
        let parsed: AskQuestionInteractionSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.questions.len(), 1);
        assert_eq!(parsed.questions[0].question, "Q1");
    }

    // =========================================================================
    // Trigger types
    // =========================================================================

    #[test]
    fn test_trigger_delivery_serde() {
        let td = TriggerDelivery::SendImmediately;
        let json = serde_json::to_string(&td).unwrap();
        assert_eq!(json, "\"send_immediately\"");
        let parsed: TriggerDelivery = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, TriggerDelivery::SendImmediately);
    }

    #[test]
    fn test_file_change_kind_serde() {
        for (kind, expected) in [
            (FileChangeKind::Added, "\"added\""),
            (FileChangeKind::Modified, "\"modified\""),
            (FileChangeKind::Deleted, "\"deleted\""),
        ] {
            assert_eq!(serde_json::to_string(&kind).unwrap(), expected);
        }
    }

    #[test]
    fn test_file_change() {
        let fc = FileChange {
            kind: FileChangeKind::Modified,
            path: "/tmp/test.rs".to_string(),
        };
        let json = serde_json::to_string(&fc).unwrap();
        let parsed: FileChange = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.kind, FileChangeKind::Modified);
        assert_eq!(parsed.path, "/tmp/test.rs");
    }

    // =========================================================================
    // Streaming types
    // =========================================================================

    #[test]
    fn test_thought_serde() {
        let t = Thought {
            step_index: 0,
            text: "I should use view_file".to_string(),
            signature: None,
        };
        let json = serde_json::to_string(&t).unwrap();
        let parsed: Thought = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.text, "I should use view_file");
    }

    #[test]
    fn test_text_chunk_serde() {
        let t = Text {
            step_index: 1,
            text: "Hello".to_string(),
        };
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("Hello"));
    }

    // =========================================================================
    // Content primitives
    // =========================================================================

    #[test]
    fn test_guess_mime_type() {
        assert_eq!(guess_mime_type(Path::new("photo.jpg")), "image/jpeg");
        assert_eq!(guess_mime_type(Path::new("photo.jpeg")), "image/jpeg");
        assert_eq!(guess_mime_type(Path::new("img.png")), "image/png");
        assert_eq!(guess_mime_type(Path::new("img.webp")), "image/webp");
        assert_eq!(guess_mime_type(Path::new("img.bmp")), "image/bmp");
        assert_eq!(guess_mime_type(Path::new("doc.pdf")), "application/pdf");
        assert_eq!(guess_mime_type(Path::new("data.json")), "application/json");
        assert_eq!(guess_mime_type(Path::new("style.css")), "text/css");
        assert_eq!(guess_mime_type(Path::new("data.csv")), "text/csv");
        assert_eq!(guess_mime_type(Path::new("page.html")), "text/html");
        assert_eq!(guess_mime_type(Path::new("page.htm")), "text/html");
        assert_eq!(guess_mime_type(Path::new("app.js")), "text/javascript");
        assert_eq!(guess_mime_type(Path::new("readme.txt")), "text/plain");
        assert_eq!(guess_mime_type(Path::new("doc.xml")), "text/xml");
        assert_eq!(guess_mime_type(Path::new("audio.wav")), "audio/wav");
        assert_eq!(guess_mime_type(Path::new("audio.mp3")), "audio/mp3");
        assert_eq!(guess_mime_type(Path::new("audio.ogg")), "audio/ogg");
        assert_eq!(guess_mime_type(Path::new("video.mp4")), "video/mp4");
        assert_eq!(guess_mime_type(Path::new("video.mov")), "video/quicktime");
        assert_eq!(
            guess_mime_type(Path::new("unknown.xyz")),
            "application/octet-stream"
        );
        assert_eq!(
            guess_mime_type(Path::new("noext")),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_content_primitive_from_str() {
        let cp = ContentPrimitive::from("hello");
        match cp {
            ContentPrimitive::Text(s) => assert_eq!(s, "hello"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_content_primitive_from_string() {
        let cp = ContentPrimitive::from("world".to_string());
        match cp {
            ContentPrimitive::Text(s) => assert_eq!(s, "world"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_content_from_str() {
        let content = Content::from("hello");
        match content {
            Content::Single(ContentPrimitive::Text(s)) => assert_eq!(s, "hello"),
            _ => panic!("Expected Single Text"),
        }
    }

    #[test]
    fn test_content_from_string() {
        let content = Content::from("test".to_string());
        match content {
            Content::Single(ContentPrimitive::Text(s)) => assert_eq!(s, "test"),
            _ => panic!("Expected Single Text"),
        }
    }

    #[test]
    fn test_content_multiple() {
        let content = Content::Multiple(vec![
            ContentPrimitive::Text("first".to_string()),
            ContentPrimitive::Text("second".to_string()),
        ]);
        match content {
            Content::Multiple(parts) => assert_eq!(parts.len(), 2),
            _ => panic!("Expected Multiple"),
        }
    }

    #[test]
    fn test_supported_image_mimes() {
        assert!(SUPPORTED_IMAGE_MIMES.contains(&"image/jpeg"));
        assert!(SUPPORTED_IMAGE_MIMES.contains(&"image/png"));
        assert!(SUPPORTED_IMAGE_MIMES.contains(&"image/webp"));
        assert!(SUPPORTED_IMAGE_MIMES.contains(&"image/bmp"));
        assert_eq!(SUPPORTED_IMAGE_MIMES.len(), 4);
    }

    #[test]
    fn test_supported_document_mimes() {
        assert!(SUPPORTED_DOCUMENT_MIMES.contains(&"application/pdf"));
        assert!(SUPPORTED_DOCUMENT_MIMES.contains(&"text/plain"));
        assert!(SUPPORTED_DOCUMENT_MIMES.contains(&"application/json"));
    }

    // =========================================================================
    // Gemini config serde roundtrip
    // =========================================================================

    #[test]
    fn test_gemini_config_serde_roundtrip() {
        let config = GeminiConfig {
            api_key: Some("test-key".to_string()),
            models: ModelConfig::default(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: GeminiConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_model_entry_serde_roundtrip() {
        let entry = ModelEntry {
            name: "gemini-2.0-pro".to_string(),
            api_key: Some("key-abc".to_string()),
            generation: GenerationConfig {
                thinking_level: Some(ThinkingLevel::High),
            },
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: ModelEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "gemini-2.0-pro");
        assert_eq!(parsed.api_key.as_deref(), Some("key-abc"));
        assert_eq!(parsed.generation.thinking_level, Some(ThinkingLevel::High));
    }

    // =========================================================================
    // System instructions
    // =========================================================================

    #[test]
    fn test_custom_system_instructions_serde() {
        let si = CustomSystemInstructions {
            text: "You are a helpful assistant.".to_string(),
        };
        let json = serde_json::to_string(&si).unwrap();
        assert!(json.contains("You are a helpful assistant."));
    }

    #[test]
    fn test_templated_system_instructions() {
        let si = TemplatedSystemInstructions {
            identity: Some("CodeBot".to_string()),
            sections: vec![SystemInstructionSection {
                content: "Follow best practices".to_string(),
                title: "guidelines".to_string(),
            }],
        };
        assert_eq!(si.identity.as_deref(), Some("CodeBot"));
        assert_eq!(si.sections.len(), 1);
    }

    // =========================================================================
    // QuestionHookResult
    // =========================================================================

    #[test]
    fn test_question_hook_result_serde() {
        let result = QuestionHookResult {
            responses: vec![QuestionResponse {
                selected_option_ids: Some(vec!["opt-1".to_string()]),
                freeform_response: String::new(),
                skipped: false,
            }],
            cancelled: false,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: QuestionHookResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.responses.len(), 1);
        assert!(!parsed.cancelled);
    }
}
