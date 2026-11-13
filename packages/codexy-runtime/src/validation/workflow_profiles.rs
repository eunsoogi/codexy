use std::path::Path;

use super::workflow_profile_evidence::{current_active_lines, has_strict_work_signal};

pub(super) fn check(_plugin_root: &Path) -> Vec<String> {
    // The invariant floor remains separate, including every governed file at or below 250 LOC with no exceptions.
    Vec::new()
}

pub(super) fn check_evidence(_plugin_root: &Path, evidence: &str) -> Vec<String> {
    // Concrete risk and collaboration still require ownership evidence, not a tier.
    let active = current_active_lines(&evidence.to_ascii_lowercase());
    let lines = active.iter().map(String::as_str).collect::<Vec<_>>();
    let collaboration_signal = has_collaboration_signal(&lines);
    let risk_requires_ownership =
        has_explicit_audit_signal(&lines) || has_strict_work_signal(&lines);
    let mut errors = Vec::new();
    if (risk_requires_ownership || collaboration_signal)
        && super::child_lane_classification_setup::formal_classification_complete_index_before(
            &lines,
            lines.len(),
        )
        .is_none()
    {
        let message = if risk_requires_ownership {
            "material-risk evidence requires the formal orchestration contract"
        } else {
            "delegation and multi-lane evidence require the formal ownership contract"
        };
        errors.push(message.to_owned());
    }
    errors
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
