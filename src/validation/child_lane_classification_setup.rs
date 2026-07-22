use super::child_lane_classification_authority::lane_authority_context_before;
use super::child_lane_classification_boundaries::current_lane_start;
use super::child_lane_classification_control::normalized_metadata_lines;
use super::child_lane_classification_fields::ClassificationFields;
use super::child_lane_classification_setup_attribution::{
    clause_has_explicit_child_scope, matched_child_branch_or_worktree_setup_clauses,
};
use super::child_lane_classification_setup_context::child_setup_context_applies;
use super::child_lane_colon_classification_block::ColonClassificationBlock;
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
        .filter(|(index, clause)| {
            child_setup_context_applies(&lines, *index, clause_has_explicit_child_scope(clause))
        })
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

    pub(super) fn has_complete_authority_record(&self) -> bool {
        self.fields.has_complete_authority_record()
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
                if !fields.record(metadata_key(key), trimmed_value(value), None) {
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
    let (mut table, mut colon_block) = (
        GfmClassificationTable::default(),
        ColonClassificationBlock::default(),
    );
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
            colon_block = ColonClassificationBlock::default();
            continue;
        }
        let Some(fields) = seen.as_mut() else {
            continue;
        };
        match table.consume(line) {
            GfmClassificationTableEvent::Ignore => continue,
            GfmClassificationTableEvent::Invalidate => {
                colon_block.invalidate(fields);
                continue;
            }
            GfmClassificationTableEvent::Replace => {
                colon_block.replace_with_gfm(fields);
                continue;
            }
            GfmClassificationTableEvent::Record(key, value) => {
                colon_block.record_gfm(fields, metadata_key(key), trimmed_value(value), authority);
                continue;
            }
            GfmClassificationTableEvent::NotGfm => {}
        }
        colon_block.consume_colon(fields, line, authority);
    }
    classification_start
        .zip(seen)
        .map(|(start, fields)| ClassificationSnapshot {
            start,
            authority,
            fields,
        })
}

pub(super) fn line_claims_setup_before_classification(line: &str) -> bool {
    let line = trimmed_value(line);
    "before task classification|before the task classification|before task-classification|before the task-classification|before formal task classification|before the formal task classification|before formal task-classification|before the formal task-classification|before formal $task-classification|before the formal $task-classification|before formal `$task-classification`|before the formal `$task-classification`|before formal classification output|before the formal classification output|before classification|before the classification|before $task-classification|before the $task-classification"
        .split('|')
    .any(|marker| line.contains(marker))
}
