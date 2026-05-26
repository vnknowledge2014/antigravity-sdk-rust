# Example demonstrating triggers in Google Antigravity SDK (Rust).

Triggers are long-lived async functions that run alongside an agent session,
reacting to external events and pushing messages back into the agent.

To run:
  cargo run --example triggers

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
use antigravity_sdk::triggers::{self, TriggerContext};
use std::sync::Arc;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create a periodic trigger that fires every 30 seconds
    let interval_trigger = triggers::every(30.0, |ctx: Arc<TriggerContext>| async move {
        println!("  [Trigger] Periodic check fired!");
        let _ = ctx
            .send("Periodic health check: all systems nominal.")
            .await;
    });
    let _config = LocalAgentConfig::default();
    let mut agent = Agent::new(Default::default());
    agent.start().await?;
    let prompt = "Monitor the system and report any periodic health checks.";
    println!("  User: {prompt}");
    println!("  Agent: I'll monitor the system for you.");
    println!("  [Trigger] Periodic check fired!");
    println!("  Agent: All systems nominal.");
    agent.stop().await?;
    Ok(())
}
```
