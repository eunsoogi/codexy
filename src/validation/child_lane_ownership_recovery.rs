use super::child_lane_ownership_phrases::*;

pub(super) fn line_has_parent_setup_recovery(lines: &[&str], index: usize) -> bool {
    let line = lines[index];
    let Some(value) = field_value(line, "recovery") else {
        return false;
    };
    let value = value
        .is_empty()
        .then(|| next_line_bullet_value(lines, index).unwrap_or(value))
        .unwrap_or(value);
    has_parent_setup_recovery_value(value)
}

fn has_parent_setup_recovery_value(value: &str) -> bool {
    let value = trimmed_value(value);
    !has_negated_recovery_step(value)
        && (value.contains("disclosed") || value.contains("disclose"))
        && (value.contains("cleaned up")
            || value.contains("cleaned-up")
            || value.contains("preserved")
            || value.contains("preserve"))
        && value.contains("overlap")
        && (value.contains("user")
            || value.contains("other-agent")
            || value.contains("other agent"))
        && (value.contains("clean child thread")
            || (value.contains("delegated") && value.contains("child thread")))
}

fn has_negated_recovery_step(value: &str) -> bool {
    [
        "did not disclose",
        "not disclose",
        "not disclosed",
        "without disclosure",
        "without disclosing",
        "no disclosure",
        "did not clean",
        "not clean",
        "without cleanup",
        "without cleaning",
        "did not preserve",
        "not preserve",
        "not preserved",
        "without preserving",
        "did not inspect",
        "not inspect",
        "without inspecting",
        "without overlap inspection",
        "no overlap inspection",
        "did not delegate",
        "not delegate",
        "not delegated",
        "without delegation",
        "without delegating",
    ]
    .into_iter()
    .any(|marker| value.contains(marker))
}
