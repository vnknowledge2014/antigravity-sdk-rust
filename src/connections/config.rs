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

//! Configuration for the local harness connection.
//!
//! Corresponds to Python's `local_connection_config.py`.

use std::collections::HashSet;
use std::env;

use crate::connections::wire_types::localharness::harness_config::ModelConfig;
use crate::connections::wire_types::localharness::*;
use crate::hooks::policy::Policy;
use crate::types::{self, *};
use std::sync::Arc;

/// Configuration for the local harness backend.
pub struct LocalAgentConfig {
    pub gemini_config: types::GeminiConfig,
    pub capabilities: types::CapabilitiesConfig,
    pub policies: Arc<Vec<Policy>>,
    pub system_instructions: Option<types::SystemInstructions>,
    pub workspaces: Vec<String>,
    pub conversation_id: Option<String>,
    pub save_dir: Option<String>,
    pub app_data_dir: Option<String>,
    pub skills_paths: Vec<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
}

impl Default for LocalAgentConfig {
    fn default() -> Self {
        Self {
            gemini_config: types::GeminiConfig::default(),
            capabilities: types::CapabilitiesConfig::default(),
            policies: Arc::new(crate::hooks::policy::confirm_run_command()),
            system_instructions: None,
            workspaces: vec![
                env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string()),
            ],
            conversation_id: None,
            save_dir: None,
            app_data_dir: None,
            skills_paths: Vec::new(),
            model: None,
            api_key: None,
        }
    }
}

impl LocalAgentConfig {
    /// Returns the effective GeminiConfig with shorthand fields applied.
    pub fn effective_gemini_config(&self) -> types::GeminiConfig {
        let mut cfg = self.gemini_config.clone();
        if let Some(ref model) = self.model {
            cfg.models.default = types::ModelEntry::from(model.as_str());
        }
        if let Some(ref key) = self.api_key {
            cfg.api_key = Some(key.clone());
        }
        cfg
    }

    /// Builds the HarnessConfig proto for the Go harness.
    pub fn build_harness_config(&self, tool_protos: Vec<Tool>) -> HarnessConfig {
        let gemini = self.effective_gemini_config();
        let cfg = &self.capabilities;

        // Determine active tools
        let all_tools: HashSet<BuiltinTools> = BuiltinTools::all_tools().into_iter().collect();
        let active_tools: HashSet<BuiltinTools> = if let Some(ref enabled) = cfg.enabled_tools {
            enabled.iter().cloned().collect()
        } else if let Some(ref disabled) = cfg.disabled_tools {
            all_tools
                .difference(&disabled.iter().cloned().collect())
                .cloned()
                .collect()
        } else {
            all_tools
        };

        let is_enabled = |t: BuiltinTools| active_tools.contains(&t);

        let harness_side_tools = HarnessSideTools {
            subagents: Some(SubagentsConfig {
                enabled: Some(cfg.enable_subagents && is_enabled(BuiltinTools::StartSubagent)),
            }),
            find: Some(FindToolConfig {
                enabled: Some(is_enabled(BuiltinTools::FindFile)),
            }),
            user_questions: Some(UserQuestionsConfig {
                enabled: Some(is_enabled(BuiltinTools::AskQuestion)),
            }),
            run_command: Some(RunCommandToolConfig {
                enabled: Some(is_enabled(BuiltinTools::RunCommand)),
            }),
            file_edit: Some(FileEditToolConfig {
                enabled: Some(is_enabled(BuiltinTools::EditFile)),
            }),
            view_file: Some(ViewFileToolConfig {
                enabled: Some(is_enabled(BuiltinTools::ViewFile)),
            }),
            write_to_file: Some(WriteToFileToolConfig {
                enabled: Some(is_enabled(BuiltinTools::CreateFile)),
            }),
            grep_search: Some(GrepSearchToolConfig {
                enabled: Some(is_enabled(BuiltinTools::SearchDir)),
            }),
            list_dir: Some(ListDirToolConfig {
                enabled: Some(is_enabled(BuiltinTools::ListDir)),
            }),
            generate_image: Some(GenerateImageToolConfig {
                enabled: Some(is_enabled(BuiltinTools::GenerateImage)),
                model_name: Some(cfg.image_model.clone()),
            }),
            permissions: None,
        };

        let workspace_protos: Vec<Workspace> = self
            .workspaces
            .iter()
            .map(|p| Workspace {
                workspace_type: Some(workspace::WorkspaceType::FilesystemWorkspace(
                    FilesystemWorkspace {
                        directory: Some(super::local::normalize_wire_path(p)),
                    },
                )),
            })
            .collect();

        let gemini_config_wire = crate::connections::wire_types::localharness::GeminiConfig {
            api_key: Some(
                gemini
                    .models
                    .default
                    .api_key
                    .or(gemini.api_key)
                    .unwrap_or_default(),
            ),
            base_url: None,
            model_name: Some(gemini.models.default.name),
            thinking_level: Some(
                gemini
                    .models
                    .default
                    .generation
                    .thinking_level
                    .map(|l| match l {
                        ThinkingLevel::Minimal => "minimal",
                        ThinkingLevel::Low => "low",
                        ThinkingLevel::Medium => "medium",
                        ThinkingLevel::High => "high",
                    })
                    .unwrap_or("")
                    .to_string(),
            ),
            enable_url_context: None,
            enable_google_search: None,
        };

        HarnessConfig {
            cascade_id: Some(self.conversation_id.clone().unwrap_or_default()),
            model_config: Some(ModelConfig::GeminiConfig(gemini_config_wire)),
            system_instructions: None, // TODO: serialize SystemInstructions
            tools: tool_protos,
            harness_side_tools: Some(harness_side_tools),
            compaction_threshold: Some(cfg.compaction_threshold.unwrap_or(0)),
            workspaces: workspace_protos,
            skills_paths: self.skills_paths.clone(),
            finish_tool_schema_json: Some(cfg.finish_tool_schema_json.clone().unwrap_or_default()),
            initial_trajectory: None,
            app_data_dir: Some(self.app_data_dir.clone().unwrap_or_default()),
        }
    }
}

impl Clone for LocalAgentConfig {
    fn clone(&self) -> Self {
        Self {
            gemini_config: self.gemini_config.clone(),
            capabilities: self.capabilities.clone(),
            policies: self.policies.clone(),
            system_instructions: self.system_instructions.clone(),
            workspaces: self.workspaces.clone(),
            conversation_id: self.conversation_id.clone(),
            save_dir: self.save_dir.clone(),
            app_data_dir: self.app_data_dir.clone(),
            skills_paths: self.skills_paths.clone(),
            model: self.model.clone(),
            api_key: self.api_key.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = LocalAgentConfig::default();
        assert!(!cfg.workspaces.is_empty());
        assert!(cfg.conversation_id.is_none());
        assert!(cfg.api_key.is_none());
        assert!(cfg.model.is_none());
        assert!(!cfg.policies.is_empty());
    }

    #[test]
    fn test_effective_gemini_config_model_override() {
        let mut cfg = LocalAgentConfig::default();
        cfg.model = Some("gemini-2.5-pro".to_string());
        let effective = cfg.effective_gemini_config();
        assert_eq!(effective.models.default.name, "gemini-2.5-pro");
    }

    #[test]
    fn test_effective_gemini_config_api_key_override() {
        let mut cfg = LocalAgentConfig::default();
        cfg.api_key = Some("test-key-123".to_string());
        let effective = cfg.effective_gemini_config();
        assert_eq!(effective.api_key.as_deref(), Some("test-key-123"));
    }

    #[test]
    fn test_build_harness_config_default() {
        let cfg = LocalAgentConfig::default();
        let harness = cfg.build_harness_config(vec![]);
        assert!(harness.cascade_id.unwrap_or_default().is_empty());
        assert!(matches!(
            harness.model_config,
            Some(ModelConfig::GeminiConfig(_))
        ));
        assert!(harness.tools.is_empty());
        assert!(!harness.workspaces.is_empty());
    }

    #[test]
    fn test_build_harness_config_with_conversation_id() {
        let mut cfg = LocalAgentConfig::default();
        cfg.conversation_id = Some("conv-123".to_string());
        let harness = cfg.build_harness_config(vec![]);
        assert_eq!(harness.cascade_id.unwrap(), "conv-123");
    }

    #[test]
    fn test_build_harness_config_with_tools() {
        let cfg = LocalAgentConfig::default();
        let tools = vec![Tool {
            name: Some("my_tool".to_string()),
            description: Some("A test tool".to_string()),
            parameters_json_schema: Some("{}".to_string()),
            response_json_schema: None,
        }];
        let harness = cfg.build_harness_config(tools);
        assert_eq!(harness.tools.len(), 1);
        assert_eq!(harness.tools[0].name.as_deref(), Some("my_tool"));
    }
}
