use super::child_lane_classification_boundaries::{current_lane_end, current_lane_start};
use super::child_lane_classification_setup_context::child_lane_context_applies;
use super::child_lane_classification_table::{
    complete_child_classification_index, records_key, table_row,
};
use super::child_lane_ownership_phrases::{metadata_key, trimmed_value};

pub(super) fn check(evidence: &str) -> Vec<String> {
    let raw_lines = evidence.lines().collect::<Vec<_>>();
    let lines = evidence.lines().map(str::trim).collect::<Vec<_>>();
    let setup_clauses = lines
        .iter()
        .enumerate()
        .flat_map(|(index, line)| {
            matched_child_branch_or_worktree_setup_clauses(line)
                .into_iter()
                .map(move |clause| (index, clause))
        })
        .filter(|(index, _)| child_lane_context_applies(&lines, *index))
        .collect::<Vec<_>>();
    if setup_clauses.is_empty() {
        return Vec::new();
    }
    if setup_clauses.iter().any(|(setup_index, setup_clause)| {
        formal_child_classification_complete_index_before(&raw_lines, &lines, *setup_index)
            .is_none()
            || line_claims_setup_before_classification(setup_clause)
    }) {
        return vec!["child-owned lane setup evidence includes child branch/worktree setup before formal $task-classification evidence completed".to_owned()];
    }
    Vec::new()
}
fn formal_child_classification_complete_index_before(
    raw_lines: &[&str],
    lines: &[&str],
    setup_index: usize,
) -> Option<usize> {
    let lane_start = current_lane_start(lines, setup_index);
    let lane_end = current_lane_end(lines, setup_index);
    complete_child_classification_index(raw_lines, lines, lane_start, setup_index, lane_end)
}
fn matched_child_branch_or_worktree_setup_clauses(line: &str) -> Vec<&str> {
    let line = trimmed_value(line);
    if line.split_once(':').is_some_and(|(key, value)| {
        records_key(metadata_key(key)) && !line_claims_setup_before_classification(value)
    }) {
        return Vec::new();
    }
    if table_row(line).is_some_and(|(key, value)| {
        records_key(key) && !line_claims_setup_before_classification(value)
    }) {
        return Vec::new();
    }
    let clauses = line
        .split_once(':')
        .filter(|(key, _)| records_key(metadata_key(key)))
        .map(|(_, value)| value)
        .unwrap_or(line);
    setup_clauses(clauses)
        .into_iter()
        .filter(|clause| !metadata_value_describes_required_negative_setup_proof(clause))
        .filter(|clause| clause_has_child_branch_or_worktree_setup(clause))
        .collect()
}
fn setup_clauses(line: &str) -> Vec<&str> {
    let mut clauses = line
        .split(&[',', ';', '.'][..])
        .flat_map(|clause| clause.split(" but "))
        .flat_map(|clause| clause.split(" however "))
        .flat_map(|clause| clause.split(" and "))
        .map(str::trim)
        .collect::<Vec<_>>();
    if !clauses.iter().any(|clause| has_absent_child_setup(clause)) {
        clauses.push(line);
    }
    clauses
}
fn clause_has_child_branch_or_worktree_setup(line: &str) -> bool {
    (has_child_setup_actor(line)
        || has_child_setup_subject(line)
        || has_codexy_branch_setup_subject(line)
        || has_unqualified_branch_or_worktree_setup(line))
        && (has_setup_action(line) || line.contains("setup"))
        && (line.contains("branch")
            || line.contains("worktree")
            || has_codexy_branch_setup_subject(line))
        && !has_parent_setup_subject(line)
        && !has_absent_child_setup(line)
}
fn has_unqualified_branch_or_worktree_setup(line: &str) -> bool {
    (line.contains("branch") || line.contains("worktree")) && has_setup_action(line)
}
fn has_parent_setup_subject(line: &str) -> bool {
    "parent coordination|parent implementation setup|parent setup|parent-owned setup|parent owned setup|parent-created branch|parent created branch|parent-created implementation branch|parent created implementation branch|parent-created worktree|parent created worktree|orchestrator-created branch|orchestrator created branch|orchestrator-created implementation branch|orchestrator created implementation branch|orchestrator-created worktree|orchestrator created worktree|orchestrator-created implementation worktree|orchestrator created implementation worktree"
        .split('|')
        .any(|marker| line.contains(marker))
}
fn has_child_setup_actor(line: &str) -> bool {
    "child created|child-created|child thread created|child-thread created|child-thread-created|child lane created|child-lane created|child-lane-created|owning child thread created|owning child lane created|child switched|child checked out|child checkout|child thread switched|child-thread switched|child thread checked out|child-thread checked out|child thread checkout|child-thread checkout|child lane switched|child-lane switched|child lane checked out|child-lane checked out|child lane checkout|child-lane checkout|created child branch|ran git worktree add|child set up|owning child thread switched|owning child thread checked out|owning child thread checkout|owning child lane switched|owning child lane checked out|owning child lane checkout"
        .split('|')
        .any(|marker| line.contains(marker))
}
fn has_child_setup_subject(line: &str) -> bool {
    "child branch|child-branch|child worktree|child-worktree|child thread branch|child-thread branch|child thread worktree|child-thread worktree|child lane branch|child-lane branch|child lane worktree|child-lane worktree|branch creation"
        .split('|')
        .any(|marker| line.contains(marker))
}
fn has_codexy_branch_setup_subject(line: &str) -> bool {
    let line = line.replace(['`', '"', '\''], "");
    ((line.contains("branch codexy/") || line.contains("branch: codexy/"))
        && has_setup_action(&line))
        || ((line.contains("worktree for codexy/") || line.contains("worktree: codexy/"))
            && has_setup_action(&line))
        || (line.contains("git worktree add") && line.contains(" codexy/"))
        || (line.contains(" codexy/") && has_setup_action(&line))
}
fn has_setup_action(line: &str) -> bool {
    "created | created:| created.| created,| created;|-created | was created| got created|creation occurred|switched | switched:| switched.| switched,| switched;| was switched| checked out| checkout |git switch | setup| set up|worktree add"
        .split('|')
        .any(|marker| line.contains(marker))
}
fn has_absent_child_setup(line: &str) -> bool {
    let line = trimmed_value(line);
    (has_child_setup_subject(line)
        && (starts_with_absent_child_setup(line) || has_negated_setup_action(line)))
        || "no child created|no child-created|not child created|not child-created|without child created|without child-created"
        .split('|')
        .any(|marker| line.contains(marker))
}
fn starts_with_absent_child_setup(line: &str) -> bool {
    "no child branch|no child-branch|no child worktree|no child-worktree|not child branch|not child-branch|not child worktree|not child-worktree|without child branch|without child-branch|without child worktree|without child-worktree|neither child branch|neither child worktree|never child branch|never child worktree|none child branch|none child worktree"
        .split('|')
        .any(|marker| line.starts_with(marker))
}
fn has_negated_setup_action(line: &str) -> bool {
    "was not created|were not created|wasn't created|weren't created|was never created|were never created|was not set up|were not set up|wasn't set up|weren't set up"
        .split('|')
        .any(|marker| line.contains(marker))
}
fn line_claims_setup_before_classification(line: &str) -> bool {
    let line = trimmed_value(line);
    "before task classification|before the task classification|before task-classification|before the task-classification|before formal task classification|before the formal task classification|before formal task-classification|before the formal task-classification|before formal $task-classification|before the formal $task-classification|before formal `$task-classification`|before the formal `$task-classification`|before formal classification output|before the formal classification output|before classification|before the classification|before $task-classification|before the $task-classification"
        .split('|')
    .any(|marker| line.contains(marker))
}
fn metadata_value_describes_required_negative_setup_proof(value: &str) -> bool {
    let value = trimmed_value(value);
    let has_negative_marker = value.contains("did not occur")
        || value.contains("didn't occur")
        || "negative test for|negative tests for|red regression for|regression test for|regression coverage for|no child branch/worktree setup occurred"
            .split('|')
            .any(|marker| value.contains(marker));
    has_negative_marker && line_claims_setup_before_classification(value)
}
