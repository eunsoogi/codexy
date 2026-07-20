use super::child_lane_classification_boundaries::{
    child_table_owns_handoff_pr, classification_owner_before, classifications,
    is_legacy_ownership_boundary, owner_at, table_ownership_boundary,
};
use super::child_lane_classification_setup::child_candidate_requires_guard;
use super::child_lane_owner_decision::{
    is_affirmative_child_owned_value, is_child_delegation_owner_decision,
};
use super::child_lane_ownership_fixes::line_has_parent_authored_fix;
use super::child_lane_ownership_phrases::*;
use super::child_lane_ownership_recovery::line_has_parent_setup_recovery;
use super::child_lane_ownership_setup::line_has_parent_implementation_setup;
pub(super) fn check(evidence: &str) -> Vec<String> {
    let normalized = evidence.to_ascii_lowercase();
    let mut errors = super::child_lane_thread_tools::check(&normalized, evidence);
    errors.extend(super::child_lane_classification_setup::check(&normalized));
    if has_unreported_worktree_mismatch_before_goal(&normalized) {
        errors.push("worktree mismatch must be reported before goal continuation".to_owned());
    }
    if has_unreassigned_parent_authored_fix(&normalized) {
        errors.push("child-owned lane contains parent-authored implementation or review-response evidence without explicit maintainer reassignment; parent implementation setup evidence is also a workflow defect".to_owned());
    }
    errors
}
fn has_unreported_worktree_mismatch_before_goal(evidence: &str) -> bool {
    let mut mismatch = false;
    for line in evidence.lines() {
        if line.contains("restart audit:")
            && line.contains("task cwd=")
            && line.contains("canonical reserved worktree=")
        {
            let values = line.split("task cwd=").nth(1).and_then(|tail| {
                let cwd = tail.split(';').next()?;
                let canonical = tail
                    .split("canonical reserved worktree=")
                    .nth(1)?
                    .split(';')
                    .next()?;
                Some((cwd, canonical))
            });
            mismatch |= values.is_some_and(|(cwd, canonical)| cwd != canonical);
        }
        if line.contains("mismatch reported before goal continuation") {
            mismatch = false;
        }
        if mismatch && line.starts_with("goal tool call:") {
            return true;
        }
    }
    false
}
fn has_unreassigned_parent_authored_fix(evidence: &str) -> bool {
    let lines = evidence.lines().map(str::trim).collect::<Vec<_>>();
    let tables = classifications(evidence);
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
        let metadata = metadata_key(line);
        let table_owner = owner_at(&tables, index);
        let starts_lane = table_owner.is_some_and(is_child_delegation_owner_decision)
            || is_affirmative_child_owned_line(line);
        let pr_metadata = line
            .split_once(':')
            .is_some_and(|(key, _)| metadata_key(key) == "pr");
        let table_owns_following_pr = child_table_owns_handoff_pr(&tables, &lines, index);
        let pr_boundary = pr_metadata
            && !table_owns_following_pr
            && index > 0
            && (previous_non_empty_line(&lines, index)
                .is_some_and(|previous| !is_affirmative_child_owned_line(previous))
                || (child_owned && child_header_open && child_pr_seen))
            && !(child_owned && child_header_open && !child_pr_seen);
        let ownership_boundary = table_owner.is_some()
            || is_legacy_ownership_boundary(line)
            || table_ownership_boundary(&tables, &lines, index);
        let line_parent_setup = line_has_parent_implementation_setup(&lines, index);
        let line_parent_fix = line_has_parent_authored_fix(&lines, index);
        let line_reassigned = line_has_explicit_maintainer_reassignment(&lines, index);
        let line_setup_recovered = line_has_parent_setup_recovery(&lines, index);
        if child_candidate_requires_guard(&tables, &lines, index)
            && (line_parent_fix || line_parent_setup)
        {
            return true;
        }
        if classification_owner_before(&lines, &tables, index)
            .is_some_and(|owner| owner.starts_with("external/human-owned"))
            && metadata.starts_with("review response:")
            && line.contains("child-authored")
        {
            return true;
        }
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
fn is_affirmative_child_owned_line(line: &str) -> bool {
    has_present_child_owner_metadata(line)
        || field_value(line, "owner").is_some_and(is_affirmative_child_owned_value)
        || field_value(line, "owner decision").is_some_and(is_child_delegation_owner_decision)
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
