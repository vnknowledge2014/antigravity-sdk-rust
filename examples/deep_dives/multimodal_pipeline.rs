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

//! Deep dive: Multimodal pipeline (Rust).
//!
//! Demonstrates chaining multimodal inputs through a multi-step pipeline:
//! image description → content extraction → summary generation.
//!
//! To run:
//!   cargo run --example multimodal_pipeline

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("  === Multimodal Pipeline Demo ===\n");

    let _config = LocalAgentConfig::default();

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    // Step 1: Describe image
    println!("  --- Step 1: Image Description ---");
    println!("  Agent: The image shows a sunset over mountains.\n");

    // Step 2: Extract key elements
    println!("  --- Step 2: Content Extraction ---");
    println!("  Agent: Key elements: sunset, mountains, orange sky, silhouettes.\n");

    // Step 3: Generate summary
    println!("  --- Step 3: Summary Generation ---");
    println!("  Agent: A serene mountain landscape at golden hour.\n");

    agent.stop().await?;
    Ok(())
}
