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
    matches!(
        normalize_metadata_prefix(line),
        "goal tool call: create_goal" | "plan tool call: update_plan"
    )
}

pub(super) fn normalize_metadata_prefix(line: &str) -> &str {
    let line = without_metadata_prefix(line);
    let line = line
        .strip_prefix("[ ] ")
        .or_else(|| line.strip_prefix("[x] "))
        .or_else(|| line.strip_prefix("[X] "))
        .unwrap_or(line);
    line
}

pub(super) fn normalized_metadata_lines<'a>(
    lines: &[&'a str],
    end: usize,
) -> (Vec<&'a str>, usize) {
    let lane_start = lines
        .iter()
        .take(end)
        .rposition(|line| {
            let normalized = normalize_metadata_prefix(line);
            normalized != line.trim_start() && normalized.starts_with("lane ownership:")
        })
        .map_or(0, |index| index + 1);
    (
        lines
            .iter()
            .map(|line| normalize_metadata_prefix(line))
            .collect(),
        lane_start,
    )
}
