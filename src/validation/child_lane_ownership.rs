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
    let (mut child_owned, mut parent_fix, mut reassigned) = (false, false, false);
    let mut pending_parent_fix = Some(false);
    let mut pending_reassigned = Some(false);
    for (index, line) in lines.iter().enumerate() {
        let starts_lane = is_affirmative_child_owned_line(line);
        let pr_boundary = line.starts_with("pr:")
            && index > 0
            && previous_non_empty_line(&lines, index)
                .is_some_and(|previous| !is_affirmative_child_owned_line(previous));
        let ownership_boundary = line.contains("ownership:");
        let line_parent_fix = line_has_parent_authored_fix(&lines, index);
        let line_reassigned = line_has_explicit_maintainer_reassignment(&lines, index);
        if (starts_lane || pr_boundary || ownership_boundary) && child_owned {
            if parent_fix && !reassigned {
                return true;
            }
            (child_owned, parent_fix, reassigned) = (false, false, false);
        }
        if pr_boundary {
            pending_parent_fix = Some(false);
            pending_reassigned = Some(false);
        } else if ownership_boundary && !starts_lane {
            pending_parent_fix = None;
            pending_reassigned = None;
        }
        if starts_lane {
            child_owned = true;
            parent_fix |= pending_parent_fix.unwrap_or(false);
            reassigned |= pending_reassigned.unwrap_or(false);
            pending_parent_fix = Some(false);
            pending_reassigned = Some(false);
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
fn is_affirmative_child_owned_line(line: &str) -> bool {
    field_value(line, "owner").is_some_and(is_affirmative_child_owned_value)
        || field_value(line, "child-owned")
            .is_some_and(|value| !has_absent_field_value(value, "child-owned"))
        || matches!(trimmed_value(line), "child-owned" | "child-owned lane")
}
fn is_affirmative_child_owned_value(value: &str) -> bool {
    let value = trimmed_value(value);
    value.contains("child-owned")
        && !value.contains("not child-owned")
        && !value.starts_with("parent-owned")
        && !has_absent_field_value(value, "child-owned")
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
        .then(|| next_line_bullet_value(lines, index).unwrap_or(value))
        .unwrap_or(value);
    is_positive_reassignment_value(value) && !is_negative_reassignment_value(value)
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
    (line.contains("parent implemented")
        || line.contains("parent fixed")
        || line.contains("parent pushed")
        || line.contains("parent implementation commit")
        || line.contains("fixed in parent")
        || line.contains("parent patched")
        || line.contains("orchestrator patched")
        || (line.contains("orchestrator authored")
            && has_fix_marker(line)
            && !has_absent_actor_phrase(line, "orchestrator", "authored"))
        || line.contains("orchestrator fixed")
        || line.contains("orchestrator review-response")
        || line.contains("orchestrator review response")
        || line.contains("parent review-response")
        || line.contains("parent review response")
        || (line.contains("parent commit") && !has_absent_actor_phrase(line, "parent", "commit"))
        || has_passive_parent_fix(line)
        || line.contains("patched by parent"))
        && !has_negative_field_value(line, "parent")
}
