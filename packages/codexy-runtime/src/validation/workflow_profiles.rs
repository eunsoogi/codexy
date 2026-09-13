use std::path::Path;

use super::workflow_profile_evidence::{current_active_lines, field_value, has_strict_work_signal};

const PROFILES: [&str; 3] = ["light", "standard", "strict"];
const DEFAULT_PROFILE: &str = "light";

pub(super) fn check(_plugin_root: &Path) -> Vec<String> {
    // The invariant floor remains separate, including every governed file at or below 250 LOC with no exceptions.
    Vec::new()
}

pub(super) fn check_evidence(_plugin_root: &Path, evidence: &str) -> Vec<String> {
    let active = current_active_lines(&evidence.to_ascii_lowercase());
    let lines = active.iter().map(String::as_str).collect::<Vec<_>>();
    let (mut profile, explicit, selection_error) = profile(&lines);
    let collaboration_signal = has_collaboration_signal(&lines);
    let strict_required =
        has_explicit_audit_signal(&lines) || has_strict_work_signal(&lines) || profile == "strict";
    if strict_required && !explicit && selection_error.is_none() {
        "strict".clone_into(&mut profile);
    }
    let mut errors = Vec::new();
    if let Some(error) = selection_error {
        errors.push(error);
    }
    if strict_required && explicit && profile != "strict" {
        errors.push("strict workflow signals require the strict workflow profile".to_owned());
    }
    if (strict_required || collaboration_signal)
        && super::child_lane_classification_setup::formal_classification_complete_index_before(
            &lines,
            lines.len(),
        )
        .is_none()
    {
        let message = if strict_required {
            "strict workflow evidence requires the formal orchestration contract"
        } else {
            "delegation and multi-lane evidence require the formal ownership contract"
        };
        errors.push(message.to_owned());
    }
    errors
}

fn profile(lines: &[&str]) -> (String, bool, Option<String>) {
    let selected = lines
        .iter()
        .filter_map(|line| field_value(line, "workflow profile"))
        .collect::<Vec<_>>();
    match selected.as_slice() {
        [] => (DEFAULT_PROFILE.to_owned(), false, None),
        [value] if PROFILES.contains(value) => ((*value).to_owned(), true, None),
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

fn has_explicit_audit_signal(lines: &[&str]) -> bool {
    lines.iter().any(|line| {
        let prefix = "explicit audit evidence:";
        line.starts_with(prefix) && affirmative(line.strip_prefix(prefix))
    })
}

fn has_collaboration_signal(lines: &[&str]) -> bool {
    lines.iter().any(|line| {
        ["durable delegation:", "multi-lane ownership:"]
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
