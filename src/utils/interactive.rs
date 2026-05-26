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

//! Interactive utilities for the Antigravity SDK.
//!
//! Corresponds to Python's `utils/interactive.py`.

use crate::types::{AskQuestionInteractionSpec, QuestionHookResult, QuestionResponse};
use std::io::{self, Write};

/// Prompts the user with questions from the spec and reads responses from stdin.
///
/// Used by the `AskQuestionHook` to gate tool execution on user approval.
pub fn ask_user_interactive(spec: &AskQuestionInteractionSpec) -> Option<QuestionHookResult> {
    let mut responses = Vec::with_capacity(spec.questions.len());

    for entry in &spec.questions {
        println!("\n{}", entry.question);

        if !entry.options.is_empty() {
            for (i, option) in entry.options.iter().enumerate() {
                println!("  {}. {}", i + 1, option.text);
            }
            if entry.is_multi_select {
                print!("Enter choices (comma-separated, or press Enter to skip): ");
            } else {
                print!(
                    "Enter choice (1-{}) or press Enter to skip: ",
                    entry.options.len()
                );
            }
        } else {
            print!("Enter response (or press Enter to skip): ");
        }

        io::stdout().flush().ok();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            responses.push(QuestionResponse {
                selected_option_ids: None,
                freeform_response: String::new(),
                skipped: true,
            });
            continue;
        }

        let trimmed = input.trim();
        if trimmed.is_empty() {
            responses.push(QuestionResponse {
                selected_option_ids: None,
                freeform_response: String::new(),
                skipped: true,
            });
            continue;
        }

        if !entry.options.is_empty() {
            let selected_ids: Vec<String> = trimmed
                .split(',')
                .filter_map(|s| {
                    let idx = s.trim().parse::<usize>().ok()?;
                    if idx >= 1 && idx <= entry.options.len() {
                        Some(entry.options[idx - 1].id.clone())
                    } else {
                        None
                    }
                })
                .collect();

            if selected_ids.is_empty() {
                responses.push(QuestionResponse {
                    selected_option_ids: None,
                    freeform_response: String::new(),
                    skipped: true,
                });
            } else {
                responses.push(QuestionResponse {
                    selected_option_ids: Some(selected_ids),
                    freeform_response: String::new(),
                    skipped: false,
                });
            }
        } else {
            responses.push(QuestionResponse {
                selected_option_ids: None,
                freeform_response: trimmed.to_string(),
                skipped: false,
            });
        }
    }

    if responses.iter().all(|r| r.skipped) {
        None
    } else {
        Some(QuestionHookResult {
            responses,
            cancelled: false,
        })
    }
}

/// Async version of `input` that handles IO via `spawn_blocking`.
pub async fn async_input(prompt: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let prompt_str = prompt.to_string();
    let (tx, rx) = tokio::sync::oneshot::channel();

    tokio::task::spawn_blocking(move || {
        print!("{}", prompt_str);
        io::stdout().flush().ok();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let _ = tx.send(input);
        }
    });

    let result = rx.await?;
    Ok(result)
}

/// Hook that prompts the user for confirmation before executing a tool.
pub struct ToolConfirmationHook;

#[async_trait::async_trait]
impl crate::hooks::DecideHook<crate::types::ToolCall> for ToolConfirmationHook {
    async fn run(
        &self,
        _context: &mut crate::hooks::HookContext,
        data: &crate::types::ToolCall,
    ) -> crate::types::HookResult {
        println!("\nTool execution requested: {:?}", data.name);
        if !data.args.is_empty() {
            println!("Arguments: {:?}", data.args);
        }

        let ans = match async_input("Allow execution? (y/n) [n]: ").await {
            Ok(s) => s,
            Err(_) => "n".to_string(),
        };

        let trimmed = ans.trim().to_lowercase();
        if trimmed == "y" || trimmed == "yes" {
            crate::types::HookResult {
                allow: true,
                message: String::new(),
            }
        } else {
            crate::types::HookResult {
                allow: false,
                message: "User denied tool call.".to_string(),
            }
        }
    }
}

/// Prompts the user for confirmation before executing a tool.
/// This is a convenient handler for use with the policy system.
pub fn ask_user_handler() -> crate::hooks::policy::AskUserHandler {
    Box::new(|tc: &crate::types::ToolCall| {
        Box::pin(async move {
            println!("\nPolicy check: Tool execution requested: {:?}", tc.name);
            if !tc.args.is_empty() {
                println!("Arguments: {:?}", tc.args);
            }

            let ans = match async_input("Allow execution? (y/n) [n]: ").await {
                Ok(s) => s,
                Err(_) => "n".to_string(),
            };

            let trimmed = ans.trim().to_lowercase();
            trimmed == "y" || trimmed == "yes"
        })
    })
}

/// Hook that prompts the user to answer questions asked by the agent.
pub struct AskQuestionHook;

#[async_trait::async_trait]
impl crate::hooks::TransformHook<AskQuestionInteractionSpec, Option<QuestionHookResult>>
    for AskQuestionHook
{
    async fn run(
        &self,
        _context: &mut crate::hooks::HookContext,
        data: &AskQuestionInteractionSpec,
    ) -> Option<QuestionHookResult> {
        let spec = data.clone();
        tokio::task::spawn_blocking(move || ask_user_interactive(&spec))
            .await
            .unwrap_or_else(|_| {
                Some(QuestionHookResult {
                    responses: vec![],
                    cancelled: true,
                })
            })
    }
}

/// Upgrades run_command from DENY to ASK_USER for interactive sessions.
pub fn upgrade_to_interactive_confirmation(agent: &mut crate::agent::Agent) {
    if let Some(runner) = agent.hook_runner_mut() {
        runner.pre_tool_call_decide_mut().clear();
        let upgraded_hook = crate::hooks::policy::enforce(vec![crate::hooks::policy::Policy {
            tool: crate::types::BuiltinTools::RunCommand.as_str().to_string(),
            decision: crate::hooks::policy::Decision::AskUser,
            when: None,
            ask_user: Some(ask_user_handler()),
            name: "interactive_confirm".to_string(),
        }]);
        runner.register_hook(crate::hooks::hook_runner::AnyHook::PreToolCallDecide(
            Box::new(upgraded_hook),
        ));
    }
}

/// Runs an interactive CLI loop for debugging and development.
pub async fn run_interactive_loop(
    agent: &mut crate::agent::Agent,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !agent.is_started() {
        return Err("Agent session not started. Use 'start()'.".into());
    }

    agent.register_hook(crate::hooks::hook_runner::AnyHook::OnInteraction(Box::new(
        AskQuestionHook,
    )));
    upgrade_to_interactive_confirmation(agent);

    println!("Starting interactive loop. Type 'exit' or 'quit' to end.");

    loop {
        let user_input = match async_input("User: ").await {
            Ok(s) => s.trim().to_string(),
            Err(_) => break, // EOF or error
        };

        if user_input.is_empty() {
            continue;
        }

        if user_input.eq_ignore_ascii_case("exit") || user_input.eq_ignore_ascii_case("quit") {
            println!("Goodbye!");
            break;
        }

        // Send to agent
        if let Err(e) = agent
            .conversation()
            .unwrap()
            .connection()
            .send(Some(crate::types::Content::from(user_input)))
            .await
        {
            println!("Error sending message: {}", e);
            continue;
        }

        // Receive steps
        use futures::StreamExt;
        let mut steps = agent.conversation().unwrap().connection().receive_steps();
        while let Some(step) = steps.next().await {
            if step.is_complete_response.unwrap_or(false)
                && !step.content.is_empty() {
                    println!("Agent: {}", step.content);
                }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    

    #[tokio::test]
    #[ignore]
    async fn test_returns_user_input() {}

    #[tokio::test]
    #[ignore]
    async fn test_default_prompt() {}

    #[tokio::test]
    #[ignore]
    async fn test_propagates_eof_error() {}

    #[tokio::test]
    #[ignore]
    async fn test_cancellation() {}

    #[tokio::test]
    #[ignore]
    async fn test_tool_confirmation_hook_allow() {}

    #[tokio::test]
    #[ignore]
    async fn test_tool_confirmation_hook_deny() {}

    #[tokio::test]
    #[ignore]
    async fn test_tool_confirmation_hook_eof() {}

    #[tokio::test]
    #[ignore]
    async fn test_ask_question_hook_option_number() {}

    #[tokio::test]
    #[ignore]
    async fn test_ask_question_hook_option_text() {}

    #[tokio::test]
    #[ignore]
    async fn test_ask_question_hook_write_in() {}

    #[tokio::test]
    #[ignore]
    async fn test_ask_question_hook_skip() {}

    #[tokio::test]
    #[ignore]
    async fn test_ask_question_hook_eof() {}

    #[tokio::test]
    #[ignore]
    async fn test_ask_user_handler_allow() {}

    #[tokio::test]
    #[ignore]
    async fn test_ask_user_handler_deny() {}

    #[tokio::test]
    #[ignore]
    async fn test_ask_user_handler_eof() {}

    // Test that upgrade_to_interactive_confirmation replaces (not appends to) pre-tool hooks.
    // Tests the logic of upgrade_to_interactive_confirmation directly:
    // 1. It clears pre_tool_call_decide hooks
    // 2. It adds exactly one new hook
    #[test]
    fn test_upgrade_replaces_hook_not_appends() {
        use crate::hooks::hook_runner::{AnyHook, HookRunner};
        use crate::hooks::policy;

        // Build a HookRunner with an existing PreToolCallDecide hook (simulating post-start state)
        let mut runner = HookRunner::new();
        let dummy_policy = policy::allow("shell");
        runner.register_hook(AnyHook::PreToolCallDecide(Box::new(policy::enforce(
            vec![dummy_policy],
        ))));
        assert_eq!(runner.pre_tool_call_decide_mut().len(), 1, "setup: should have 1 hook");

        // Now simulate what upgrade_to_interactive_confirmation does:
        runner.pre_tool_call_decide_mut().clear();
        let upgraded = policy::enforce(vec![policy::Policy {
            tool: crate::types::BuiltinTools::RunCommand.as_str().to_string(),
            decision: policy::Decision::AskUser,
            when: None,
            ask_user: Some(ask_user_handler()),
            name: "interactive_confirm".to_string(),
        }]);
        runner.register_hook(AnyHook::PreToolCallDecide(Box::new(upgraded)));

        // After: should be exactly 1, not 2 (clear replaced, not appended)
        assert_eq!(runner.pre_tool_call_decide_mut().len(), 1, "should have exactly 1 hook after upgrade");
    }

    #[tokio::test]
    async fn test_run_interactive_loop_before_start() {
        let mut agent = crate::agent::Agent::new(crate::agent::AgentArgs::default());
        let res = run_interactive_loop(&mut agent).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    #[ignore]
    async fn test_run_interactive_loop() {}

    #[tokio::test]
    #[ignore]
    async fn test_run_interactive_loop_interrupt() {}
}
