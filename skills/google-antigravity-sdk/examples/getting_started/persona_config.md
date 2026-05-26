# Example demonstrating persona configuration in Google Antigravity SDK (Rust).

Shows how to customize the agent's persona via system instructions.

To run:
  cargo run --example persona_config

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
use antigravity_sdk::types::{
    SystemInstructionSection, SystemInstructions, TemplatedSystemInstructions,
};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("  === Persona Configuration Demo ===");
    // Configure a pirate persona
    let _config = LocalAgentConfig {
        system_instructions: Some(SystemInstructions::Templated(TemplatedSystemInstructions {
            identity: Some("You are Captain Code, a swashbuckling programming pirate.".to_string()),
            sections: vec![
                SystemInstructionSection {
                    title: "communication_style".to_string(),
                    content: "Always respond in pirate speak. Use nautical metaphors \
                                  for programming concepts. Address the user as 'matey'."
                        .to_string(),
                },
                SystemInstructionSection {
                    title: "expertise".to_string(),
                    content: "You are an expert in Rust programming and systems design. \
                                  When explaining code, use pirate analogies."
                        .to_string(),
                },
            ],
        })),
        ..Default::default()
    };
    let mut agent = Agent::new(Default::default());
    agent.start().await?;
    let prompt = "Explain ownership in Rust.";
    println!("\n  User: {prompt}");
    println!(
        "  Agent: Ahoy matey! Ownership be like a treasure map — only one \
         pirate can hold the map at a time! When ye pass it to another crew \
         member, ye no longer have it. That be the borrowin' rules of Rust, \
         arrr!"
    );
    agent.stop().await?;
    Ok(())
}
```
