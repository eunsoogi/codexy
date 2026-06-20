use super::child_lane_ownership_phrases::*;

pub(super) fn line_has_parent_setup_recovery(lines: &[&str], index: usize) -> bool {
    let line = lines[index];
    let Some(value) = field_value(line, "recovery") else {
        return false;
    };
    if value.is_empty() {
        return recovery_continuation_value(lines, index)
            .is_some_and(|value| has_parent_setup_recovery_value(&value));
    }
    has_parent_setup_recovery_value(value)
}

fn recovery_continuation_value(lines: &[&str], index: usize) -> Option<String> {
    let mut value = String::new();
    for line in lines.iter().skip(index + 1) {
        if line.is_empty() {
            continue;
        }
        let continuation = continuation_value(line);
        if is_recovery_continuation_boundary(continuation) {
            break;
        }
        if !value.is_empty() {
            value.push_str("; ");
        }
        value.push_str(continuation);
    }
    (!value.is_empty()).then_some(value)
}

fn continuation_value(value: &str) -> &str {
    value.trim_start_matches(['-', '*']).trim()
}

fn is_recovery_continuation_boundary(value: &str) -> bool {
    value.split_once(':').is_some_and(|(key, _)| {
        let key = metadata_key(key);
        [
            "branch",
            "child owner",
            "head",
            "implementation surface reads",
            "implementation-surface reads",
            "lane owner",
            "lane ownership",
            "maintainer reassignment",
            "orchestrator implementation setup",
            "owner",
            "parent implementation setup",
            "pr",
            "pr ownership",
            "pull request ownership",
            "recovery",
            "review response",
            "review-response",
            "worktree path",
        ]
        .into_iter()
        .any(|field| key == field || key.contains(field))
    })
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
