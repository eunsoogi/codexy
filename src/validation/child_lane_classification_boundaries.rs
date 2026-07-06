use super::child_lane_ownership_phrases::{metadata_key, trimmed_value};

pub(super) fn current_lane_start(lines: &[&str], setup_index: usize) -> usize {
    (0..setup_index)
        .rev()
        .find(|index| is_lane_boundary(lines, *index))
        .map_or(0, |index| index + 1)
}

fn is_lane_boundary(lines: &[&str], index: usize) -> bool {
    let line = metadata_key(trimmed_value(lines[index]));
    if "pr:|pull request:"
        .split('|')
        .any(|marker| line.starts_with(marker))
    {
        return true;
    }
    if line.starts_with("lane ownership:") {
        return !is_after_task_classification_block(lines, index);
    }
    is_owner_metadata(line) && !is_inside_task_classification(lines, index)
}

fn is_owner_metadata(line: &str) -> bool {
    "owner:|child owner:|lane owner:|owner decision:"
        .split('|')
        .any(|marker| line.starts_with(marker))
}

fn is_inside_task_classification(lines: &[&str], index: usize) -> bool {
    for line in lines
        .iter()
        .take(index)
        .rev()
        .map(|line| metadata_key(trimmed_value(line)))
    {
        if line.is_empty() {
            continue;
        }
        if line == "task classification:" {
            return true;
        }
        if is_lane_boundary_terminator(line) || is_hard_lane_boundary(line) {
            return false;
        }
        if !is_task_classification_field(line) {
            return false;
        }
    }
    false
}

fn is_hard_lane_boundary(line: &str) -> bool {
    "pr:|pull request:|lane ownership:"
        .split('|')
        .any(|marker| line.starts_with(marker))
}

fn is_after_task_classification_block(lines: &[&str], index: usize) -> bool {
    for line in lines
        .iter()
        .take(index)
        .rev()
        .map(|line| metadata_key(trimmed_value(line)))
    {
        if line.is_empty() {
            continue;
        }
        if line == "task classification:" {
            return true;
        }
        if is_lane_boundary_terminator(line) || is_hard_lane_boundary(line) {
            return false;
        }
        if !is_task_classification_field(line) {
            return false;
        }
    }
    false
}

fn is_lane_boundary_terminator(line: &str) -> bool {
    "review response:|maintainer reassignment:"
        .split('|')
        .any(|marker| line.starts_with(marker))
}

fn is_task_classification_field(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, _)| {
        matches!(
            super::child_lane_ownership_phrases::metadata_key(key),
            "lane type"
                | "secondary surfaces"
                | "owner decision"
                | "atomic scope"
                | "required skills"
                | "required tools/evidence"
                | "required tools"
                | "required evidence"
                | "first allowed action"
                | "stop/blocker"
                | "stop blocker"
                | "blocker"
        )
    })
}
