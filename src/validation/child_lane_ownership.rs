use super::child_lane_ownership_phrases::*;

pub(super) fn check(evidence: &str) -> Vec<String> {
    let normalized = evidence.to_lowercase();
    if has_unreassigned_parent_authored_fix(&normalized) {
        return vec![
            "child-owned lane contains parent-authored implementation or review-response evidence without explicit maintainer reassignment".to_owned(),
        ];
    }
    Vec::new()
}
fn has_unreassigned_parent_authored_fix(evidence: &str) -> bool {
    let lines = evidence.lines().map(str::trim).collect::<Vec<_>>();
    let (mut child_owned, mut parent_fix, mut reassigned, mut child_header_open) =
        (false, false, false, false);
    let mut pending_parent_fix = Some(false);
    let mut pending_reassigned = Some(false);
    let mut pending_pr_seen = false;
    for (index, line) in lines.iter().enumerate() {
        let starts_lane = is_affirmative_child_owned_line(line);
        let pr_metadata = is_pr_boundary(line);
        let pr_boundary = pr_metadata
            && index > 0
            && previous_non_empty_line(&lines, index)
                .is_some_and(|previous| !is_affirmative_child_owned_line(previous))
            && !(child_owned && child_header_open);
        let ownership_boundary =
            is_lane_ownership_boundary(line) || is_parent_owned_owner_boundary(line);
        let line_parent_fix = line_has_parent_authored_fix(&lines, index);
        let line_reassigned = line_has_explicit_maintainer_reassignment(&lines, index);
        if (starts_lane || pr_boundary || ownership_boundary) && child_owned {
            if parent_fix && !reassigned {
                return true;
            }
            (child_owned, parent_fix, reassigned, child_header_open) = (false, false, false, false);
        }
        if pr_boundary {
            if pending_pr_seen || !pending_parent_fix.unwrap_or(false) {
                pending_parent_fix = Some(false);
                pending_reassigned = Some(false);
            }
        } else if ownership_boundary && !starts_lane {
            pending_parent_fix = None;
            pending_reassigned = None;
            pending_pr_seen = false;
        }
        pending_pr_seen |= pr_metadata;
        if starts_lane {
            child_owned = true;
            child_header_open = true;
            parent_fix |= pending_parent_fix.unwrap_or(false);
            reassigned |= pending_reassigned.unwrap_or(false);
            pending_parent_fix = Some(false);
            pending_reassigned = Some(false);
            pending_pr_seen = false;
        }
        if child_owned {
            parent_fix |= line_parent_fix;
            reassigned |= line_reassigned;
        } else if let Some(pending) = pending_parent_fix.as_mut() {
            *pending |= line_parent_fix;
            if let Some(pending) = pending_reassigned.as_mut() {
                *pending |= line_reassigned;
            }
        }
        if child_header_open && !is_child_lane_header_metadata(line) {
            child_header_open = false;
        }
    }
    child_owned && parent_fix && !reassigned
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
fn line_has_parent_authored_fix(lines: &[&str], index: usize) -> bool {
    let line = lines[index];
    if field_value(line, "parent-authored").is_some_and(str::is_empty)
        && next_line_bullet_value(lines, index).is_some_and(has_absent_value)
    {
        return false;
    }
    if line.contains("parent-authored")
        && !has_negative_field_value(line, "parent-authored")
        && !has_absent_authored_phrase(line, "parent-authored")
        && !has_nested_absent_authored_field(line, "parent-authored")
        && !has_draft_handoff_phrase(line, "parent-authored")
    {
        return has_fix_marker(line) || has_affirmative_implementation_field(line);
    }
    if line.contains("parent authored")
        && !has_negative_field_value(line, "parent")
        && !has_absent_actor_phrase(line, "parent", "authored")
    {
        return has_fix_marker(line) || has_affirmative_implementation_field(line);
    }
    if line.contains("orchestrator-authored")
        && !has_negative_field_value(line, "orchestrator-authored")
        && !has_absent_authored_phrase(line, "orchestrator-authored")
        && !has_nested_absent_authored_field(line, "orchestrator-authored")
        && !has_draft_handoff_phrase(line, "orchestrator-authored")
    {
        return has_fix_marker(line) || has_affirmative_implementation_field(line);
    }
    if line.contains("orchestrator authored")
        && !has_negative_field_value(line, "orchestrator")
        && !has_absent_actor_phrase(line, "orchestrator", "authored")
    {
        return has_fix_marker(line) || has_affirmative_implementation_field(line);
    }
    (has_present_actor_action(line, "parent", "implemented")
        || has_present_actor_action(line, "orchestrator", "implemented")
        || has_present_actor_action(line, "parent", "fixed")
        || has_present_actor_action(line, "parent", "pushed")
        || has_present_actor_action(line, "orchestrator", "pushed")
        || has_present_actor_action(line, "parent", "implementation commit")
        || has_present_actor_action(line, "orchestrator", "implementation commit")
        || has_present_fixed_in_parent_phrase(line)
        || has_present_actor_action(line, "parent", "patched")
        || has_present_actor_action(line, "orchestrator", "patched")
        || (line.contains("orchestrator authored")
            && has_fix_marker(line)
            && !has_absent_actor_phrase(line, "orchestrator", "authored"))
        || has_present_actor_action(line, "orchestrator", "fixed")
        || has_present_actor_action(line, "orchestrator", "review-response")
        || has_present_actor_action(line, "orchestrator", "review response")
        || has_present_actor_action(line, "parent", "review-response")
        || has_present_actor_action(line, "parent", "review response")
        || has_present_actor_action(line, "parent", "commit")
        || has_present_actor_action(line, "orchestrator", "commit")
        || has_passive_parent_fix(line)
        || line.contains("patched by parent"))
        && !has_negative_field_value(line, "parent")
}
fn has_present_actor_action(line: &str, actor: &str, marker: &str) -> bool {
    let field = format!("{actor} {marker}");
    line.contains(&field)
        && !has_negative_field_value(line, &field)
        && !has_absent_actor_phrase(line, actor, marker)
}
fn has_present_fixed_in_parent_phrase(line: &str) -> bool {
    line.contains("fixed in parent") && !has_absent_fixed_in_parent_phrase(line)
}
fn has_absent_fixed_in_parent_phrase(line: &str) -> bool {
    ["no ", "not ", "without "]
        .into_iter()
        .map(|prefix| format!("{prefix}fixed in parent"))
        .any(|absent_marker| {
            let Some(index) = line.find(&absent_marker) else {
                return false;
            };
            let after_absence = &line[index + absent_marker.len()..];
            !after_absence.contains("fixed in parent")
        })
}
