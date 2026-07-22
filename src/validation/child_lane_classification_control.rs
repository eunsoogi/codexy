use super::child_lane_classification_setup::formal_child_classification_complete_index_before;
use super::child_lane_classification_setup_context::prior_child_lane_context_applies;
use super::child_terminal_handoff::without_metadata_prefix;

pub(super) fn check(evidence: &str) -> Vec<String> {
    let lines = evidence.lines().map(str::trim).collect::<Vec<_>>();
    if lines.iter().enumerate().any(|(index, line)| {
        is_control_call(line)
            && prior_child_lane_context_applies(&lines, index)
            && formal_child_classification_complete_index_before(&lines, index).is_none()
    }) {
        return vec!["child-owned lane control evidence includes create_goal or update_plan before formal $task-classification evidence completed".to_owned()];
    }
    Vec::new()
}

fn is_control_call(line: &str) -> bool {
    let line = without_metadata_prefix(line);
    let line = line
        .strip_prefix("[ ] ")
        .or_else(|| line.strip_prefix("[x] "))
        .or_else(|| line.strip_prefix("[X] "))
        .unwrap_or(line);
    matches!(
        line,
        "goal tool call: create_goal" | "plan tool call: update_plan"
    )
}
