use std::path::Path;

use toml::Value;

use crate::paths::display_relative;

const PROFILE_REVIEWERS: &[&str] = &["codexy-inspector", "codexy-sentinel"];
const REQUIRED_MARKERS: &[&str] = &[
    "MUST prioritize the smallest underlying structural defect, ownership error, invalid state model, or violated invariant",
    "MUST consolidate same-cause examples into one finding with representative counterexamples and one structural repair boundary",
    "genuinely distinct correctness, security, permission, data-loss, and compatibility defects separate",
];
const PROHIBITED_INSTRUCTIONS: &[&str] = &[
    "must reward phrase-by-phrase edge-case hunting",
    "must collapse genuinely distinct invariants",
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
