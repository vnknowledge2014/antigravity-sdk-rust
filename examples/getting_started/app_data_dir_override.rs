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

//! Example demonstrating app_data_dir override (Rust).
//!
//! To run:
//!   cargo run --example app_data_dir_override

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;
use tempfile::tempdir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let temp_dir_obj = tempdir()?;
    let custom_dir = temp_dir_obj.path().to_string_lossy().to_string();

    let _config = LocalAgentConfig {
        app_data_dir: Some(custom_dir.clone()),
        ..Default::default()
    };

    println!("  === App Data Dir Override Demo ===");
    println!("  Custom app_data_dir: {custom_dir}");

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    println!("  Agent: Using custom app data directory.");

    agent.stop().await?;
    Ok(())
}
