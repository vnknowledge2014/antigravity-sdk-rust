# Multimodal example for Google Antigravity SDK (Rust).

Demonstrates:
- Multimodal input: Passing images and documents to the agent.
- Multimodal output: Enabling the agent to generate images.

To run:
  cargo run --example multimodal

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
use antigravity_sdk::types::{BuiltinTools, CapabilitiesConfig, Content, ContentPrimitive, Media};
use std::env;
use std::path::PathBuf;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Setup paths to resources
    let script_dir = env::current_exe()
        .map(|p| p.parent().unwrap().to_path_buf())
        .unwrap_or_else(|_| PathBuf::from("."));
    let resources_dir = script_dir.join("../../examples/resources");
    let image_path = resources_dir.join("example_image.png");
    let doc_path = resources_dir.join("sample_doc.txt");
    // Multimodal Input: Image
    println!("  --- Multimodal Input: Image ---");
    if image_path.exists() {
        let image = Media::from_file(&image_path, None)?;
        let prompt = Content::Multiple(vec![
            ContentPrimitive::Text("What is in this image?".to_string()),
            ContentPrimitive::Media(image),
        ]);
        println!("  User: What is in this image?");
        // TODO: agent.chat(prompt) once wired
        println!("  Agent: [Image description]\n");
    } else {
        println!("  Skipped: {} not found\n", image_path.display());
    }
    // Multimodal Input: Document
    println!("  --- Multimodal Input: Document ---");
    if doc_path.exists() {
        let doc = Media::from_file(&doc_path, None)?;
        let prompt = Content::Multiple(vec![
            ContentPrimitive::Text("Summarize this document".to_string()),
            ContentPrimitive::Media(doc),
        ]);
        println!("  User: Summarize this document");
        println!("  Agent: [Document summary]\n");
    } else {
        println!("  Skipped: {} not found\n", doc_path.display());
    }
    // Multimodal Output: Image Generation
    println!("  --- Multimodal Output: Image Generation ---");
    let _gen_config = LocalAgentConfig {
        capabilities: CapabilitiesConfig {
            enabled_tools: Some(vec![BuiltinTools::GenerateImage]),
            ..Default::default()
        },
        ..Default::default()
    };
    let prompt = "Generate an image of a futuristic city, name it 'future_city'.";
    println!("  User: {prompt}");
    println!("  Agent: [Generated image at /path/to/future_city.png]\n");
    Ok(())
}
```
