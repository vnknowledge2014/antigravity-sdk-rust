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

//! Helper factories for common trigger patterns.
//!
//! Corresponds to Python's `triggers/helpers.py`.

use std::future::Future;
use std::sync::Arc;

use crate::triggers::triggers::{Trigger, TriggerContext};

/// Creates a trigger that runs callback on a fixed interval.
pub fn every<F, Fut>(interval_secs: f64, callback: F) -> Trigger
where
    F: Fn(Arc<TriggerContext>) -> Fut + Send + Sync + Clone + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    Box::new(move |ctx: Arc<TriggerContext>| {
        let ctx = ctx.clone();
        let callback = callback.clone();
        let duration = std::time::Duration::from_secs_f64(interval_secs);
        Box::pin(async move {
            loop {
                tokio::time::sleep(duration).await;
                callback(ctx.clone()).await;
            }
        })
    })
}

/// Creates a trigger that fires once after a delay.
pub fn after<F, Fut>(delay_secs: f64, callback: F) -> Trigger
where
    F: Fn(Arc<TriggerContext>) -> Fut + Send + Sync + Clone + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    Box::new(move |ctx: Arc<TriggerContext>| {
        let ctx = ctx.clone();
        let callback = callback.clone();
        let duration = std::time::Duration::from_secs_f64(delay_secs);
        Box::pin(async move {
            tokio::time::sleep(duration).await;
            callback(ctx).await;
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_every_creates_trigger() {
        let trigger = every(1.0, |_ctx| async {});
        // Type check — trigger should be a valid Trigger
        let _: Trigger = trigger;
    }

    #[test]
    fn test_after_creates_trigger() {
        let trigger = after(1.0, |_ctx| async {});
        let _: Trigger = trigger;
    }

    #[test]
    fn test_every_zero_interval() {
        // Should not panic at creation time
        let _trigger = every(0.0, |_ctx| async {});
    }

    #[test]
    fn test_every_fractional_interval() {
        let _trigger = every(0.5, |_ctx| async {});
    }
}
