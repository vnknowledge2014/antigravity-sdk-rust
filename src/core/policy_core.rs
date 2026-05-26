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

//! Pure policy evaluation logic.
//!
//! Functions moved from `hooks/policy.rs` to the functional core.
//! These are referentially transparent — same inputs always produce same outputs.

/// Priority bucket constants for policy evaluation.
///
/// Policies are grouped into 6 priority levels:
/// Specific Deny > Specific Ask > Specific Allow >
/// Wildcard Deny > Wildcard Ask > Wildcard Allow
pub const LEVEL_SPECIFIC_DENY: usize = 0;
pub const LEVEL_SPECIFIC_ASK: usize = 1;
pub const LEVEL_SPECIFIC_ALLOW: usize = 2;
pub const LEVEL_WILDCARD_DENY: usize = 3;
pub const LEVEL_WILDCARD_ASK: usize = 4;
pub const LEVEL_WILDCARD_ALLOW: usize = 5;
pub const NUM_LEVELS: usize = 6;
pub const WILDCARD: &str = "*";

use crate::hooks::policy::{Decision, Policy};

/// Pure: Determine which priority bucket a policy belongs to.
pub fn bucket_index(p: &Policy) -> usize {
    let is_wildcard = p.tool == WILDCARD;
    match (is_wildcard, p.decision) {
        (false, Decision::Deny) => LEVEL_SPECIFIC_DENY,
        (false, Decision::AskUser) => LEVEL_SPECIFIC_ASK,
        (false, Decision::Approve) => LEVEL_SPECIFIC_ALLOW,
        (true, Decision::Deny) => LEVEL_WILDCARD_DENY,
        (true, Decision::AskUser) => LEVEL_WILDCARD_ASK,
        (true, Decision::Approve) => LEVEL_WILDCARD_ALLOW,
    }
}

/// Pure: Check if a policy matches a given tool name.
pub fn matches_tool(policy: &Policy, tool_name: &str) -> bool {
    policy.tool == WILDCARD || policy.tool == tool_name
}

/// Pure: Sort policies into priority buckets.
///
/// Returns a vector of 6 buckets (one per priority level),
/// each containing the policies assigned to that level.
pub fn build_buckets(policies: Vec<Policy>) -> Vec<Vec<Policy>> {
    let mut buckets: Vec<Vec<Policy>> = (0..NUM_LEVELS).map(|_| Vec::new()).collect();
    for p in policies {
        let idx = bucket_index(&p);
        buckets[idx].push(p);
    }
    buckets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::policy::{allow, deny};

    #[test]
    fn test_bucket_index_specific_deny() {
        assert_eq!(bucket_index(&deny("foo")), LEVEL_SPECIFIC_DENY);
    }

    #[test]
    fn test_bucket_index_wildcard_allow() {
        assert_eq!(bucket_index(&allow("*")), LEVEL_WILDCARD_ALLOW);
    }

    #[test]
    fn test_bucket_index_wildcard_deny() {
        assert_eq!(bucket_index(&deny("*")), LEVEL_WILDCARD_DENY);
    }

    #[test]
    fn test_matches_tool_exact() {
        let p = allow("my_tool");
        assert!(matches_tool(&p, "my_tool"));
        assert!(!matches_tool(&p, "other"));
    }

    #[test]
    fn test_matches_tool_wildcard() {
        let p = allow("*");
        assert!(matches_tool(&p, "any_tool"));
    }

    #[test]
    fn test_build_buckets_empty() {
        let buckets = build_buckets(vec![]);
        assert_eq!(buckets.len(), NUM_LEVELS);
        assert!(buckets.iter().all(|b| b.is_empty()));
    }

    #[test]
    fn test_build_buckets_sorts_correctly() {
        let policies = vec![
            allow("*"),           // Wildcard Allow -> bucket 5
            deny("dangerous"),    // Specific Deny -> bucket 0
            allow("safe"),        // Specific Allow -> bucket 2
        ];
        let buckets = build_buckets(policies);
        assert_eq!(buckets[LEVEL_SPECIFIC_DENY].len(), 1);
        assert_eq!(buckets[LEVEL_SPECIFIC_ALLOW].len(), 1);
        assert_eq!(buckets[LEVEL_WILDCARD_ALLOW].len(), 1);
        // Other buckets empty
        assert!(buckets[LEVEL_SPECIFIC_ASK].is_empty());
        assert!(buckets[LEVEL_WILDCARD_DENY].is_empty());
        assert!(buckets[LEVEL_WILDCARD_ASK].is_empty());
    }

    #[test]
    fn test_build_buckets_preserves_order() {
        let policies = vec![deny("a"), deny("b"), deny("c")];
        let buckets = build_buckets(policies);
        let names: Vec<_> = buckets[LEVEL_SPECIFIC_DENY]
            .iter()
            .map(|p| p.tool.as_str())
            .collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }
}
