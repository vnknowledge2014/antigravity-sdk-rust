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

//! Pure step and history operations.
//!
//! Functions for manipulating conversation steps and usage metadata
//! without side effects.

use crate::types::UsageMetadata;

/// Pure: Add two Option<i32> values, treating None as absent.
pub fn add_option(a: Option<i32>, b: Option<i32>) -> Option<i32> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a + b),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// Pure: Merge new usage data into an existing accumulator.
///
/// Returns a new `UsageMetadata` without mutating either input.
/// When `existing` is `None`, returns a clone of `new`.
pub fn merge_usage(existing: Option<&UsageMetadata>, new: &UsageMetadata) -> UsageMetadata {
    match existing {
        Some(e) => UsageMetadata {
            prompt_token_count: add_option(e.prompt_token_count, new.prompt_token_count),
            cached_content_token_count: add_option(
                e.cached_content_token_count,
                new.cached_content_token_count,
            ),
            candidates_token_count: add_option(
                e.candidates_token_count,
                new.candidates_token_count,
            ),
            thoughts_token_count: add_option(e.thoughts_token_count, new.thoughts_token_count),
            total_token_count: add_option(e.total_token_count, new.total_token_count),
        },
        None => new.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_option_both_some() {
        assert_eq!(add_option(Some(10), Some(20)), Some(30));
    }

    #[test]
    fn test_add_option_left_some() {
        assert_eq!(add_option(Some(10), None), Some(10));
    }

    #[test]
    fn test_add_option_right_some() {
        assert_eq!(add_option(None, Some(20)), Some(20));
    }

    #[test]
    fn test_add_option_both_none() {
        assert_eq!(add_option(None, None), None);
    }

    #[test]
    fn test_merge_usage_from_none() {
        let new = UsageMetadata {
            prompt_token_count: Some(100),
            candidates_token_count: Some(50),
            total_token_count: Some(150),
            ..Default::default()
        };
        let result = merge_usage(None, &new);
        assert_eq!(result.prompt_token_count, Some(100));
        assert_eq!(result.candidates_token_count, Some(50));
        assert_eq!(result.total_token_count, Some(150));
    }

    #[test]
    fn test_merge_usage_accumulates() {
        let existing = UsageMetadata {
            prompt_token_count: Some(100),
            candidates_token_count: Some(50),
            total_token_count: Some(150),
            ..Default::default()
        };
        let new = UsageMetadata {
            prompt_token_count: Some(200),
            candidates_token_count: Some(75),
            total_token_count: Some(275),
            ..Default::default()
        };
        let result = merge_usage(Some(&existing), &new);
        assert_eq!(result.prompt_token_count, Some(300));
        assert_eq!(result.candidates_token_count, Some(125));
        assert_eq!(result.total_token_count, Some(425));
    }

    #[test]
    fn test_merge_usage_is_pure() {
        let existing = UsageMetadata {
            prompt_token_count: Some(100),
            ..Default::default()
        };
        let new = UsageMetadata {
            prompt_token_count: Some(50),
            ..Default::default()
        };
        let _result = merge_usage(Some(&existing), &new);
        // Inputs unchanged — pure function guarantee
        assert_eq!(existing.prompt_token_count, Some(100));
        assert_eq!(new.prompt_token_count, Some(50));
    }
}
