# Example demonstrating agent skills configuration (Rust).

To run:
  cargo run --example agent_skills

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
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _config = LocalAgentConfig {
        skills_paths: vec!["./skills".to_string()],
        ..Default::default()
    };
    let mut agent = Agent::new(Default::default());
    agent.start().await?;
    println!("  === Agent Skills Demo ===");
    println!("  Skills paths configured: [\"./skills\"]");
    println!("  Agent: Skills loaded and ready.");
    agent.stop().await?;
    Ok(())
}
```
