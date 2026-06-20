use super::child_lane_ownership_phrases::{
    field_value, has_absent_authored_phrase, has_absent_field_value, trimmed_value,
};

const SETUP_ARTIFACT_MARKERS: &str = "parent-created draft worktree|parent-created implementation worktree|parent-created implementation branch|parent created draft worktree|parent created implementation worktree|parent created implementation branch|orchestrator-created draft worktree|orchestrator-created implementation worktree|orchestrator-created implementation branch|orchestrator created draft worktree|orchestrator created implementation worktree|orchestrator created implementation branch";
const GENERIC_SETUP_ARTIFACT_MARKERS: &str =
    "created draft worktree|created implementation worktree|created implementation branch";

pub(super) fn line_has_present_setup_artifact(line: &str) -> bool {
    value_has_present_setup_artifact(trimmed_value(line))
}

pub(super) fn value_has_present_setup_artifact(value: &str) -> bool {
    value_has_present_actor_setup_artifact(value)
        || GENERIC_SETUP_ARTIFACT_MARKERS
            .split('|')
            .any(|marker| has_present_generic_setup_artifact_marker(value, marker))
}

fn value_has_present_actor_setup_artifact(value: &str) -> bool {
    SETUP_ARTIFACT_MARKERS
        .split('|')
        .any(|marker| value.contains(marker) && !has_absent_setup_marker(value, marker))
}

pub(super) fn clause_has_absent_setup_artifact_marker(clause: &str) -> bool {
    SETUP_ARTIFACT_MARKERS
        .split('|')
        .any(|marker| clause.contains(marker) && has_absent_setup_marker(clause, marker))
}

fn has_present_generic_setup_artifact_marker(value: &str, marker: &str) -> bool {
    value.match_indices(marker).any(|(index, _)| {
        let prefix = &value[..index];
        !has_explicit_non_parent_setup_actor_prefix(prefix)
            && !has_absent_setup_marker(value, marker)
    })
}

fn has_explicit_non_parent_setup_actor_prefix(prefix: &str) -> bool {
    [
        "child-",
        "child ",
        "child thread ",
        "child-thread ",
        "child-thread-",
        "child lane ",
        "child-lane ",
        "child-lane-",
        "child-owned thread ",
        "child-owned lane ",
        "child owned thread ",
        "child owned lane ",
        "owning child thread ",
        "owning child lane ",
        "parent-",
        "parent ",
        "orchestrator-",
        "orchestrator ",
    ]
    .into_iter()
    .any(|actor_prefix| prefix.ends_with(actor_prefix))
}

fn has_absent_setup_marker(line: &str, marker: &str) -> bool {
    field_value(line, marker).is_some_and(|value| has_absent_field_value(value, marker))
        || has_absent_authored_phrase(line, marker)
}
