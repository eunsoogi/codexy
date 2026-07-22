use super::child_lane_classification_authority::lane_authority_context_before;
use super::child_lane_classification_boundaries::current_lane_start;
use super::child_lane_classification_control::normalized_metadata_lines;
use super::child_lane_classification_fields::ClassificationFields;
use super::child_lane_classification_setup_context::child_lane_context_applies;
use super::child_lane_gfm_classification_table::{
    GfmClassificationTable, GfmClassificationTableEvent,
};
use super::child_lane_ownership_phrases::{metadata_key, trimmed_value};

pub(super) fn check(evidence: &str) -> Vec<String> {
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
        formal_child_classification_complete_index_before(&lines, *setup_index).is_none()
            || line_claims_setup_before_classification(setup_clause)
    }) {
        return vec!["child-owned lane setup evidence includes child branch/worktree setup before formal $task-classification evidence completed".to_owned()];
    }
    Vec::new()
}
pub(super) fn formal_child_classification_complete_index_before(
    lines: &[&str],
    setup_index: usize,
) -> Option<usize> {
    let snapshot = latest_classification_before(lines, setup_index)?;
    (snapshot
        .authority
        .is_some_and(|authority| authority.authorizes_child_setup())
        && snapshot.fields.is_complete())
    .then_some(snapshot.start)
}

pub(super) struct ClassificationSnapshot {
    start: usize,
    authority: Option<super::child_lane_classification_authority::LaneAuthority>,
    fields: ClassificationFields,
}

impl ClassificationSnapshot {
    pub(super) fn has_complete_child_display(&self) -> bool {
        self.fields.has_complete_child_display()
    }
}

pub(super) fn has_complete_gfm_display_before(lines: &[&str], end: usize) -> bool {
    let (mut fields, mut table) = (
        ClassificationFields::default(),
        GfmClassificationTable::default(),
    );
    for line in lines.iter().take(end).map(|line| line.trim()) {
        match table.consume(line) {
            GfmClassificationTableEvent::Ignore => {}
            GfmClassificationTableEvent::Invalidate => fields = ClassificationFields::default(),
            GfmClassificationTableEvent::Replace => fields = ClassificationFields::default(),
            GfmClassificationTableEvent::Record(key, value) => {
                if !fields.record(metadata_key(key), trimmed_value(value), None, true) {
                    fields = ClassificationFields::default();
                }
            }
            GfmClassificationTableEvent::NotGfm => {}
        }
    }
    fields.has_complete_child_display()
}

pub(super) fn latest_classification_before(
    lines: &[&str],
    setup_index: usize,
) -> Option<ClassificationSnapshot> {
    let (mut seen, mut authority, mut classification_start) = (None, None, None);
    let mut table = GfmClassificationTable::default();
    let raw_lines = lines;
    let (lines, prefixed_lane_start) = normalized_metadata_lines(lines, setup_index);
    let lane_start = current_lane_start(&lines, setup_index).max(prefixed_lane_start);
    for (index, line) in lines.iter().enumerate().take(setup_index).skip(lane_start) {
        if *line == "task classification:" {
            seen = Some(ClassificationFields::default());
            let context = lane_authority_context_before(raw_lines, index);
            authority = context.authority();
            classification_start = Some(index);
            table = GfmClassificationTable::default();
            continue;
        }
        let Some(fields) = seen.as_mut() else {
            continue;
        };
        match table.consume(line) {
            GfmClassificationTableEvent::Ignore => continue,
            GfmClassificationTableEvent::Invalidate => {
                *fields = ClassificationFields::default();
                continue;
            }
            GfmClassificationTableEvent::Replace => {
                *fields = ClassificationFields::default();
                continue;
            }
            GfmClassificationTableEvent::Record(key, value) => {
                if !fields.record(metadata_key(key), trimmed_value(value), authority, true) {
                    *fields = ClassificationFields::default();
                }
                continue;
            }
            GfmClassificationTableEvent::NotGfm => {}
        }
        if line.is_empty() {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = metadata_key(key);
            if ClassificationFields::records_key(key)
                && !fields.record(key, trimmed_value(value), authority, false)
            {
                *fields = ClassificationFields::default();
            }
        }
    }
    classification_start
        .zip(seen)
        .map(|(start, fields)| ClassificationSnapshot {
            start,
            authority,
            fields,
        })
}

fn matched_child_branch_or_worktree_setup_clauses(line: &str) -> Vec<&str> {
    let line = trimmed_value(line);
    if line.split_once(':').is_some_and(|(key, value)| {
        ClassificationFields::records_key(metadata_key(key))
            && !line_claims_setup_before_classification(value)
    }) {
        return Vec::new();
    }
    let clauses = line
        .split_once(':')
        .filter(|(key, _)| ClassificationFields::records_key(metadata_key(key)))
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
