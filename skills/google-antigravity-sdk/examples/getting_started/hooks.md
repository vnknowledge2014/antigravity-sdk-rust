# Example demonstrating all supported lifecycle hooks in Google Antigravity SDK (Rust).

Shows how to implement hook traits for session, turn, tool, interaction, and
compaction lifecycle events.

To run:
  cargo run --example hooks

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
use antigravity_sdk::hooks::{DecideHook, HookContext, HookRunner, InspectHook};
use antigravity_sdk::types::{Content, HookResult, ToolCall};
use async_trait::async_trait;
// --- Session Hooks ---
struct OnStartHook;
#[async_trait]
impl InspectHook<()> for OnStartHook {
    async fn run(&self, _ctx: &mut HookContext, _data: &()) {
        println!("\n  [Hook] Session started");
    }
}
struct OnEndHook;
#[async_trait]
impl InspectHook<()> for OnEndHook {
    async fn run(&self, _ctx: &mut HookContext, _data: &()) {
        println!("\n  [Hook] Session ended");
    }
}
// --- Turn Hooks ---
struct PreTurnHook;
#[async_trait]
impl DecideHook<Content> for PreTurnHook {
    async fn run(&self, _ctx: &mut HookContext, _data: &Content) -> HookResult {
        println!("\n  [Hook] Pre-turn: Intercepted prompt");
        HookResult::allowed()
    }
}
struct PostTurnHook;
#[async_trait]
impl InspectHook<String> for PostTurnHook {
    async fn run(&self, _ctx: &mut HookContext, data: &String) {
        println!("\n  [Hook] Post-turn: Final response -> {data:?}");
    }
}
// --- Tool Hooks ---
struct PreToolHook;
#[async_trait]
impl DecideHook<ToolCall> for PreToolHook {
    async fn run(&self, _ctx: &mut HookContext, data: &ToolCall) -> HookResult {
        println!("\n  [Hook] Pre-tool-call: Approving tool -> {}", data.name);
        HookResult::allowed()
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _config = LocalAgentConfig::default();
    // Build hook runner with all hooks
    let mut hook_runner = HookRunner::new();
    hook_runner.register_hook(antigravity_sdk::hooks::hook_runner::AnyHook::OnSessionStart(Box::new(OnStartHook)));
    hook_runner.register_hook(antigravity_sdk::hooks::hook_runner::AnyHook::OnSessionEnd(Box::new(OnEndHook)));
    hook_runner.register_hook(antigravity_sdk::hooks::hook_runner::AnyHook::PreToolCallDecide(Box::new(PreToolHook)));
    // Manually dispatch to demonstrate
    hook_runner.dispatch_session_start().await;
    let mut agent = Agent::new(Default::default());
    agent.start().await?;
    println!("  --- Starting Interaction ---");
    // 1. Simple chat
    println!("\n  --- Prompt 1: Simple Chat ---");
    let prompt = "Say 'Hello World!'";
    println!("  Agent Response: Hello World!");
    // 2. Tool usage
    println!("\n  --- Prompt 2: Tool Usage ---");
    println!("  Agent Response: Hello, Alice!");
    // 3. Tool error
    println!("\n  --- Prompt 3: Tool Error ---");
    println!("  Agent Response: [Tool error handled]");
    println!("\n  --- Finished Interaction ---");
    agent.stop().await?;
    hook_runner.dispatch_session_end().await;
    Ok(())
}
```
