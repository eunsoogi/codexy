use super::child_lane_ownership_fixes::line_has_parent_authored_fix;
use super::child_lane_ownership_phrases::*;
use super::child_lane_ownership_recovery::line_has_parent_setup_recovery;
use super::child_lane_ownership_setup::line_has_parent_implementation_setup;
pub(super) fn check(evidence: &str) -> Vec<String> {
    let normalized = evidence.to_lowercase();
    if has_unreassigned_parent_authored_fix(&normalized) {
        return vec!["child-owned lane contains parent-authored implementation or review-response evidence without explicit maintainer reassignment; parent implementation setup evidence is also a workflow defect".to_owned()];
    }
    Vec::new()
}
fn has_unreassigned_parent_authored_fix(evidence: &str) -> bool {
    let lines = evidence.lines().map(str::trim).collect::<Vec<_>>();
    let (
        mut child_owned,
        mut parent_fix,
        mut parent_setup,
        mut reassigned,
        mut setup_recovered,
        mut child_header_open,
        mut child_pr_seen,
    ) = (false, false, false, false, false, false, false);
    let mut pending_parent_fix = Some(false);
    let mut pending_parent_setup = Some(false);
    let mut pending_reassigned = Some(false);
    let mut pending_setup_recovered = Some(false);
    let mut pending_pr_seen = false;
    for (index, line) in lines.iter().enumerate() {
        let starts_lane = is_affirmative_child_owned_line(line);
        let pr_metadata = is_pr_boundary(line);
        let pr_boundary = pr_metadata
            && index > 0
            && (previous_non_empty_line(&lines, index)
                .is_some_and(|previous| !is_affirmative_child_owned_line(previous))
                || (child_owned && child_header_open && child_pr_seen))
            && !(child_owned && child_header_open && !child_pr_seen);
        let ownership_boundary =
            is_lane_ownership_boundary(line) || is_parent_owned_owner_boundary(line);
        let line_parent_setup = line_has_parent_implementation_setup(line);
        let line_parent_fix = line_has_parent_authored_fix(&lines, index);
        let line_reassigned = line_has_explicit_maintainer_reassignment(&lines, index);
        let line_setup_recovered = line_has_parent_setup_recovery(&lines, index);
        if (starts_lane || pr_boundary || ownership_boundary) && child_owned {
            if child_owned_lane_has_violation(parent_fix, parent_setup, reassigned, setup_recovered)
            {
                return true;
            }
            child_owned = false;
            parent_fix = false;
            parent_setup = false;
            reassigned = false;
            setup_recovered = false;
            child_header_open = false;
            child_pr_seen = false;
        }
        if pr_boundary {
            if pending_pr_seen
                || !(pending_parent_fix.unwrap_or(false) || pending_parent_setup.unwrap_or(false))
            {
                pending_parent_fix = Some(false);
                pending_parent_setup = Some(false);
                pending_reassigned = Some(false);
                pending_setup_recovered = Some(false);
            }
        } else if ownership_boundary && !starts_lane {
            pending_parent_fix = None;
            pending_parent_setup = None;
            pending_reassigned = None;
            pending_setup_recovered = None;
            pending_pr_seen = false;
        }
        pending_pr_seen |= pr_metadata;
        if starts_lane {
            child_owned = true;
            child_header_open = true;
            child_pr_seen = pending_pr_seen;
            parent_fix |= pending_parent_fix.unwrap_or(false);
            parent_setup |= pending_parent_setup.unwrap_or(false);
            reassigned |= pending_reassigned.unwrap_or(false);
            setup_recovered |= pending_setup_recovered.unwrap_or(false);
            pending_parent_fix = Some(false);
            pending_parent_setup = Some(false);
            pending_reassigned = Some(false);
            pending_setup_recovered = Some(false);
            pending_pr_seen = false;
        }
        if child_owned {
            child_pr_seen |= pr_metadata;
            parent_fix |= line_parent_fix;
            parent_setup |= line_parent_setup;
            reassigned |= line_reassigned;
            setup_recovered |= line_setup_recovered;
        } else if let Some(pending) = pending_parent_fix.as_mut() {
            *pending |= line_parent_fix;
            if let Some(pending) = pending_parent_setup.as_mut() {
                *pending |= line_parent_setup;
            }
            if let Some(pending) = pending_reassigned.as_mut() {
                *pending |= line_reassigned;
            }
            if let Some(pending) = pending_setup_recovered.as_mut() {
                *pending |= line_setup_recovered;
            }
        }
        if child_header_open && !is_child_lane_header_metadata(line) {
            child_header_open = false;
        }
    }
    child_owned
        && child_owned_lane_has_violation(parent_fix, parent_setup, reassigned, setup_recovered)
}
fn child_owned_lane_has_violation(
    parent_fix: bool,
    parent_setup: bool,
    reassigned: bool,
    setup_recovered: bool,
) -> bool {
    (parent_fix && !reassigned) || (parent_setup && !reassigned && !setup_recovered)
}
fn previous_non_empty_line<'a>(lines: &'a [&str], index: usize) -> Option<&'a str> {
    lines[..index]
        .iter()
        .rev()
        .find(|line| !line.is_empty())
        .copied()
}
fn is_pr_boundary(line: &str) -> bool {
    line.split_once(':')
        .is_some_and(|(key, _)| metadata_key(key) == "pr")
}
fn is_lane_ownership_boundary(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, _)| {
        let key = metadata_key(key);
        matches!(
            key,
            "ownership" | "lane ownership" | "pr ownership" | "pull request ownership"
        )
    })
}
fn is_child_lane_header_metadata(line: &str) -> bool {
    line.is_empty()
        || line.starts_with("pr:")
        || is_affirmative_child_owned_line(line)
        || is_exact_child_header_metadata_line(line)
}
fn is_exact_child_header_metadata_line(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, _)| {
        matches!(
            metadata_key(key),
            "owner"
                | "lane owner"
                | "branch"
                | "head"
                | "worktree path"
                | "maintainer reassignment"
        )
    })
}
fn is_parent_owned_owner_boundary(line: &str) -> bool {
    field_value(line, "owner").is_some_and(is_parent_owned_value)
}
fn is_affirmative_child_owned_line(line: &str) -> bool {
    has_present_child_owner_metadata(line)
        || field_value(line, "owner").is_some_and(is_affirmative_child_owned_value)
        || field_value(line, "child-owned")
            .is_some_and(|value| !has_absent_field_value(value, "child-owned"))
        || matches!(trimmed_value(line), "child-owned" | "child-owned lane")
}
fn has_present_child_owner_metadata(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, value)| {
        metadata_key(key) == "child owner"
            && !trimmed_value(value).is_empty()
            && !has_absent_field_value(value, "child owner")
    })
}
fn is_affirmative_child_owned_value(value: &str) -> bool {
    let value = trimmed_value(value);
    value.contains("child-owned")
        && !value.contains("not child-owned")
        && !value.starts_with("parent-owned")
        && !has_absent_field_value(value, "child-owned")
}
fn is_parent_owned_value(value: &str) -> bool {
    let value = trimmed_value(value);
    value.starts_with("parent-owned") && !value.contains("not parent-owned")
}
fn line_has_explicit_maintainer_reassignment(lines: &[&str], index: usize) -> bool {
    let line = lines[index];
    if has_non_affirmative_reassignment_key(line) {
        return false;
    }
    let Some(value) = field_value(line, "maintainer reassignment") else {
        return false;
    };
    let value = value
        .is_empty()
        .then(|| next_line_reassignment_value(lines, index).unwrap_or(value))
        .unwrap_or(value);
    is_positive_reassignment_value(value) && !is_negative_reassignment_value(value)
}
fn next_line_reassignment_value<'a>(lines: &'a [&str], index: usize) -> Option<&'a str> {
    let value = next_line_bullet_value(lines, index)?;
    (!is_non_reassignment_metadata_field(value)).then_some(value)
}
fn is_non_reassignment_metadata_field(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, _)| {
        let key = metadata_key(key);
        !key.is_empty() && !key.contains("reassignment") && !key.contains("reassigned")
    })
}
