# Example demonstrating native structured output from an agent (Rust).

Shows how to configure the agent to return strongly-typed JSON instead of
raw conversational text.

To run:
  cargo run --example structured_output

```rust
//
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

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::tools::{RegisteredTool, ToolRunner};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::future::Future;
use std::pin::Pin;
/// Represents a single action item from a meeting.
#[derive(Debug, Serialize, Deserialize)]
struct ActionItem {
    assignee: String,
    task: String,
    deadline: String,
}
/// Summarizes a meeting, including a list of action items.
#[derive(Debug, Serialize, Deserialize)]
struct MeetingSummary {
    action_items: Vec<ActionItem>,
}
/// Retrieves the raw unstructured notes for a given meeting ID.
fn fetch_unstructured_meeting_notes(
    args: Value,
) -> Pin<Box<dyn Future<Output = Result<Value, Box<dyn std::error::Error + Send + Sync>>> + Send>> {
    Box::pin(async move {
        let meeting_id = args
            .get("meeting_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if meeting_id == "meeting-2026-05" {
            Ok(json!(
                "Discussed launch timeline for project X. Alice agreed to update \
                 the textproto tests by Monday. Bob mentioned he will run the final \
                 E2E benchmarks tomorrow. I will push the release build once the \
                 tests are green."
            ))
        } else {
            Ok(json!("Error: Meeting notes not found."))
        }
    })
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("  --- Starting main ---");
    let mut tool_runner = ToolRunner::new();
    tool_runner.register(RegisteredTool {
        name: "fetch_unstructured_meeting_notes".to_string(),
        description: "Retrieves the raw unstructured notes for a given meeting ID.".to_string(),
        schema: Some(json!({
            "type": "object",
            "properties": {
                "meeting_id": {"type": "string", "description": "The meeting ID to fetch notes for."}
            },
            "required": ["meeting_id"]
        })),
        handler: Box::new(fetch_unstructured_meeting_notes),
    });
    // Generate MeetingSummary JSON Schema for response_schema
    let _schema = json!({
        "type": "object",
        "properties": {
            "action_items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "assignee": {"type": "string"},
                        "task": {"type": "string"},
                        "deadline": {"type": "string"}
                    },
                    "required": ["assignee", "task", "deadline"]
                }
            }
        },
        "required": ["action_items"]
    });
    let _config = LocalAgentConfig::default();
    let mut agent = Agent::new(Default::default());
    agent.start().await?;
    let prompt = concat!(
        "Use the fetch_unstructured_meeting_notes tool to retrieve notes for ",
        "'meeting-2026-05' and return the meeting summary with the appropriate ",
        "action item list."
    );
    println!("\n  Sending prompt to agent...");
    println!("  Prompt: {prompt}");
    // Simulate structured output
    let mock_summary = MeetingSummary {
        action_items: vec![
            ActionItem {
                assignee: "Alice".to_string(),
                task: "Update textproto tests".to_string(),
                deadline: "Monday".to_string(),
            },
            ActionItem {
                assignee: "Bob".to_string(),
                task: "Run final E2E benchmarks".to_string(),
                deadline: "Tomorrow".to_string(),
            },
        ],
    };
    println!("\n  === Structured Meeting Action Items ===");
    for item in &mock_summary.action_items {
        println!("  - Assignee: {}", item.assignee);
        println!("    Task:     {}", item.task);
        println!("    Deadline: {}\n", item.deadline);
    }
    agent.stop().await?;
    Ok(())
}
```
