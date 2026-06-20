use super::child_lane_ownership_phrases::{field_value, has_absent_field_value, trimmed_value};

pub(super) fn line_has_parent_implementation_setup(line: &str) -> bool {
    if field_value(line, "parent implementation setup")
        .or_else(|| field_value(line, "orchestrator implementation setup"))
        .or_else(|| field_value(line, "implementation-surface reads"))
        .or_else(|| field_value(line, "implementation surface reads"))
        .is_some_and(|value| !has_absent_field_value(value, "implementation setup"))
    {
        return true;
    }
    let line = trimmed_value(line);
    [
        "parent-created draft worktree",
        "parent-created implementation worktree",
        "parent-created implementation branch",
        "parent created draft worktree",
        "parent created implementation worktree",
        "parent created implementation branch",
        "orchestrator-created draft worktree",
        "orchestrator-created implementation worktree",
        "orchestrator-created implementation branch",
        "orchestrator created draft worktree",
        "orchestrator created implementation worktree",
        "orchestrator created implementation branch",
    ]
    .into_iter()
    .any(|marker| line.contains(marker))
}
