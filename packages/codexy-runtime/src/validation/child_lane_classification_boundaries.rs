use super::child_lane_ownership_phrases::{metadata_key, trimmed_value};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum LaneBoundary {
    PullRequest,
    ReviewResponse,
    MaintainerReassignment,
    Ownership,
}

impl LaneBoundary {
    pub(super) fn resets_authority_record(self) -> bool {
        !matches!(self, Self::Ownership)
    }

    pub(super) fn requires_fresh_classification(self) -> bool {
        matches!(self, Self::ReviewResponse | Self::MaintainerReassignment)
    }
}

pub(super) fn current_lane_start(lines: &[&str], setup_index: usize) -> usize {
    (0..setup_index)
        .rev()
        .find(|index| lane_boundary(lines, *index).is_some())
        .map_or(0, |index| index + 1)
}

pub(super) fn current_lane_record_start(lines: &[&str], end: usize) -> usize {
    let start = current_lane_start(lines, end);
    let Some(boundary) = start.checked_sub(1) else {
        return 0;
    };
    let line = metadata_key(trimmed_value(lines[boundary]));
    if line.starts_with("lane ownership:")
        && boundary > 0
        && metadata_key(trimmed_value(lines[boundary - 1]))
            .starts_with("ownership metadata source:")
    {
        return boundary - 1;
    }
    boundary
}

pub(super) fn lane_boundary(lines: &[&str], index: usize) -> Option<LaneBoundary> {
    let raw_line = trimmed_value(lines[index]);
    if raw_line.starts_with('|') {
        return None;
    }
    let line = metadata_key(raw_line);
    if let Some(boundary) = fixed_lane_boundary(line) {
        return Some(boundary);
    }
    if line.starts_with("lane ownership:") {
        return (!is_after_task_classification_block(lines, index))
            .then_some(LaneBoundary::Ownership);
    }
    (is_owner_metadata(line) && !is_inside_task_classification(lines, index))
        .then_some(LaneBoundary::Ownership)
}

fn is_owner_metadata(line: &str) -> bool {
    "owner:|child owner:|lane owner:|owner decision:"
        .split('|')
        .any(|marker| line.starts_with(marker))
}

pub(super) fn is_inside_task_classification(lines: &[&str], index: usize) -> bool {
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
        if fixed_lane_boundary(line).is_some() || is_hard_ownership_boundary(line) {
            return false;
        }
        if !is_task_classification_field(line) {
            return false;
        }
    }
    false
}

fn is_hard_ownership_boundary(line: &str) -> bool {
    line.starts_with("lane ownership:")
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
        if fixed_lane_boundary(line).is_some() || is_hard_ownership_boundary(line) {
            return false;
        }
        if !is_task_classification_field(line) {
            return false;
        }
    }
    false
}

fn fixed_lane_boundary(line: &str) -> Option<LaneBoundary> {
    if line.starts_with("pr:") || line.starts_with("pull request:") {
        Some(LaneBoundary::PullRequest)
    } else if line.starts_with("review response:") {
        Some(LaneBoundary::ReviewResponse)
    } else if line.starts_with("maintainer reassignment:") {
        Some(LaneBoundary::MaintainerReassignment)
    } else {
        None
    }
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
