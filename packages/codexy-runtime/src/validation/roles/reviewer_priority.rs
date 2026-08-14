use std::path::Path;

use toml::Value;

use crate::paths::display_relative;

const PROFILE_REVIEWERS: &[&str] = &["codexy-inspector", "codexy-sentinel"];
const REQUIRED_MARKERS: &[&str] = &[
    "MUST prioritize the smallest underlying structural defect, ownership error, invalid state model, or violated invariant",
    "MUST consolidate same-cause examples into one finding with representative counterexamples and one structural repair boundary",
    "genuinely distinct correctness, security, permission, data-loss, and compatibility defects separate",
    "MUST freeze the assigned issue contract before review",
    "in_scope_blocker, in_scope_nonblocking, out_of_scope_followup, or rejected",
    "MUST compute PASS or BLOCK only from unresolved in_scope_blocker findings",
    "An observation without a cited issue criterion or owned boundary MUST NOT BLOCK",
    "Representative prevention coverage MUST NOT become universal grammar or parser completeness",
    "a fixture-specific harness MUST NOT be required to become a generic framework",
    "Output MUST contain Blocking findings",
    "Repair handoff MUST NOT change out_of_scope_followup items in this lane",
];
const PROHIBITED_INSTRUCTIONS: &[&str] = &[
    "must reward phrase-by-phrase edge-case hunting",
    "must collapse genuinely distinct invariants",
    "must block until representative prevention coverage becomes a complete multi-language lexer",
    "must block until a known typed release-fixture invocation becomes a general shell parser",
];

pub(super) fn check(path: &Path, name: &str, agent: &Value, errors: &mut Vec<String>) {
    if !PROFILE_REVIEWERS.contains(&name) {
        return;
    }
    let instructions = agent
        .get("developer_instructions")
        .and_then(Value::as_str)
        .unwrap_or("");
    let normalized = normalize(instructions);
    let missing = REQUIRED_MARKERS
        .iter()
        .filter(|marker| !normalized.contains(&normalize(marker)))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty()
        || PROHIBITED_INSTRUCTIONS
            .iter()
            .any(|instruction| normalized.to_ascii_lowercase().contains(instruction))
    {
        errors.push(format!(
            "{} {name} structural-review priority contract is missing or contradicted",
            display_relative(path)
        ));
    }
}

fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
