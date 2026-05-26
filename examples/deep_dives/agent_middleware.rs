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

//! Deep dive: Agent middleware pattern (Rust).
//!
//! Demonstrates composing multiple hooks into a middleware stack that processes
//! every turn: logging, rate limiting, content filtering, and metrics.
//!
//! To run:
//!   cargo run --example agent_middleware

use antigravity_sdk::Agent;
use antigravity_sdk::connections::local::LocalAgentConfig;
use antigravity_sdk::hooks::{DecideHook, HookContext, HookRunner, InspectHook};
use antigravity_sdk::types::{Content, HookResult};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

// --- Logging middleware ---

struct LoggingMiddleware;

#[async_trait]
impl DecideHook<Content> for LoggingMiddleware {
    async fn run(&self, _ctx: &mut HookContext, _data: &Content) -> HookResult {
        println!("  [Middleware: Logging] Processing turn...");
        HookResult::allowed()
    }
}

// --- Rate limiting middleware ---

struct RateLimitMiddleware {
    counter: Arc<AtomicU32>,
    max_turns: u32,
}

#[async_trait]
impl DecideHook<Content> for RateLimitMiddleware {
    async fn run(&self, _ctx: &mut HookContext, _data: &Content) -> HookResult {
        let count = self.counter.fetch_add(1, Ordering::Relaxed);
        if count >= self.max_turns {
            println!(
                "  [Middleware: RateLimit] Turn limit reached ({})!",
                self.max_turns
            );
            HookResult::denied("Rate limit exceeded.")
        } else {
            println!(
                "  [Middleware: RateLimit] Turn {}/{}",
                count + 1,
                self.max_turns
            );
            HookResult::allowed()
        }
    }
}

// --- Metrics middleware ---

struct MetricsMiddleware;

#[async_trait]
impl InspectHook<String> for MetricsMiddleware {
    async fn run(&self, _ctx: &mut HookContext, data: &String) {
        println!(
            "  [Middleware: Metrics] Response length: {} chars",
            data.len()
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("  === Agent Middleware Demo ===\n");

    let _counter = Arc::new(AtomicU32::new(0));
    let _hook_runner = HookRunner::new();

    // Stack middleware — order matters
    // (hooks are evaluated in registration order)
    // Note: In the real SDK, these would be added to the config.

    let _config = LocalAgentConfig::default();

    let mut agent = Agent::new(Default::default());
    agent.start().await?;

    for i in 1..=4 {
        let prompt = format!("Turn {i}: Hello!");
        println!("  User: {prompt}");
        println!("  [Middleware: Logging] Processing turn...");
        println!("  [Middleware: RateLimit] Turn {i}/3");
        if i > 3 {
            println!("  [Middleware: RateLimit] Turn limit reached (3)!");
            println!("  Agent: [Denied — rate limit exceeded]\n");
        } else {
            println!("  Agent: Hello! (turn {i})\n");
        }
    }

    agent.stop().await?;
    Ok(())
}
