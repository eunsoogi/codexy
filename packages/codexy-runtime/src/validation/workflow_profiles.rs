use std::path::Path;

use serde_json::Value;

use super::workflow_profile_evidence::{current_active_lines, field_value, has_strict_work_signal};

const CONTRACT_PATH: &str = "skills/orchestration/references/workflow-profiles.json";
const PROFILES: [&str; 3] = ["light", "standard", "strict"];
const TRIGGERS: [&str; 4] = [
    "strict",
    "durable delegation",
    "multi-lane ownership",
    "explicit audit evidence",
];
const INVARIANTS: [&str; 5] = [
    "destructive-action safety",
    "user and unrelated change preservation",
    "no force push or force-with-lease",
    "current-head readiness proof",
    "every governed file at or below 250 LOC with no exceptions",
];

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    load(plugin_root)
        .and_then(validate)
        .err()
        .into_iter()
        .collect()
}

pub(super) fn check_evidence(plugin_root: &Path, evidence: &str) -> Vec<String> {
    let Ok(contract) = load(plugin_root).and_then(validate) else {
        return check(plugin_root);
    };
    let active = current_active_lines(&evidence.to_ascii_lowercase());
    let lines = active.iter().map(String::as_str).collect::<Vec<_>>();
    let (mut profile, explicit, selection_error) = profile(&contract, &lines);
    let formal_trigger =
        has_formal_trigger(&lines) || has_strict_work_signal(&lines) || profile == "strict";
    if formal_trigger && !explicit && selection_error.is_none() {
        "strict".clone_into(&mut profile);
    }
    let mut errors = Vec::new();
    if let Some(error) = selection_error {
        errors.push(error);
    }
    if formal_trigger && explicit && profile != "strict" {
        errors.push("formal evidence triggers require the strict workflow profile".to_owned());
    }
    if formal_trigger
        && super::child_lane_classification_setup::formal_classification_complete_index_before(
            &lines,
            lines.len(),
        )
        .is_none()
    {
        errors
            .push("strict workflow evidence requires the formal orchestration contract".to_owned());
    }
    errors
}

fn load(plugin_root: &Path) -> Result<Value, String> {
    let path = plugin_root.join(CONTRACT_PATH);
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("{} cannot be read: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("{} is invalid JSON: {error}", path.display()))
}

fn validate(contract: Value) -> Result<Value, String> {
    if contract["version"] != 1
        || contract["defaultProfile"] != "light"
        || contract["escalationOrder"] != serde_json::json!(PROFILES)
    {
        return Err(
            "workflow profile contract must define version 1, light default, and escalation order"
                .to_owned(),
        );
    }
    let profiles = contract["profiles"]
        .as_object()
        .ok_or("workflow profile contract profiles must be an object")?;
    if profiles.len() != PROFILES.len()
        || !PROFILES
            .iter()
            .all(|profile| profiles.contains_key(*profile))
    {
        return Err(
            "workflow profile contract must define exactly light, standard, and strict".to_owned(),
        );
    }
    if contract["profiles"]["light"]
        != serde_json::json!({
            "includes": ["read-only", "documentation", "tiny fixes", "ordinary single-owner mutations"],
            "doesNotRequire": ["formal classification table", "goal/plan receipts", "multi-agent decisions", "specialist skip rationales", "unavailable-tool explanations"],
            "requiresFormalEvidence": false
        })
        || contract["profiles"]["standard"]
            != serde_json::json!({
                "includes": ["non-trivial single-owner work"],
                "doesNotRequire": ["formal classification table"],
                "requiresFormalEvidence": false
            })
        || contract["profiles"]["strict"]
            != serde_json::json!({
                "includes": ["high-risk, security, release, multi-lane, and merge-sensitive work"],
                "requiresFormalEvidence": true
            })
        || contract["proofAndReview"]
            != serde_json::json!({
                "light": "proportionate to the requested mutation and invariant floor",
                "standard": "proportionate planning and proof for non-trivial single-owner work",
                "strict": "formal current-head proof and the applicable Sentinel review"
            })
        || contract["formalEvidenceTriggers"] != serde_json::json!(TRIGGERS)
        || contract["invariantFloor"] != serde_json::json!(INVARIANTS)
        || contract["authorizedMergeGates"]
            != serde_json::json!(["title", "label", "thread", "connector", "merge-message"])
    {
        return Err(
            "workflow profile contract is missing strict evidence triggers or invariant floor"
                .to_owned(),
        );
    }
    Ok(contract)
}

fn profile(contract: &Value, lines: &[&str]) -> (String, bool, Option<String>) {
    let selected = lines
        .iter()
        .filter_map(|line| field_value(line, "workflow profile"))
        .collect::<Vec<_>>();
    match selected.as_slice() {
        [] => (
            contract["defaultProfile"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            false,
            None,
        ),
        [value] if contract["profiles"].get(*value).is_some() => ((*value).to_owned(), true, None),
        [_] => (
            String::new(),
            true,
            Some("workflow profile must be one of light, standard, or strict".to_owned()),
        ),
        _ => (
            String::new(),
            true,
            Some("workflow profile must be declared exactly once for the current lane".to_owned()),
        ),
    }
}

fn has_formal_trigger(lines: &[&str]) -> bool {
    lines.iter().any(|line| {
        [
            "durable delegation:",
            "multi-lane ownership:",
            "explicit audit evidence:",
        ]
        .iter()
        .any(|prefix| line.starts_with(prefix) && affirmative(line.strip_prefix(prefix)))
    })
}

fn affirmative(value: Option<&str>) -> bool {
    matches!(
        value.and_then(|value| value.split_whitespace().next()),
        Some("yes" | "true" | "required" | "requested")
    )
}
