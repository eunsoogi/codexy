use super::child_lane_ownership_phrases::*;

pub(super) fn line_has_parent_authored_fix(lines: &[&str], index: usize) -> bool {
    let line = lines[index];
    if field_value(line, "parent-authored").is_some_and(str::is_empty)
        && next_line_bullet_value(lines, index)
            .is_some_and(|value| has_absent_value(value) || is_metadata_field(value))
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
