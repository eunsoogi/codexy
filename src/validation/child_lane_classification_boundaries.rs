use super::child_lane_classification_evidence::{ClassificationEvidence, ClassificationTable};
use super::child_lane_owner_decision::{is_child_delegation_owner_decision, is_parent_owned_value};
use super::child_lane_ownership_phrases::{field_value, metadata_key, trimmed_value};

pub(super) fn current_lane_start(lines: &[&str], setup_index: usize) -> usize {
    (0..setup_index)
        .rev()
        .find(|index| is_lane_boundary(lines, *index))
        .map_or(0, |index| index + 1)
}

pub(super) fn next_lane_boundary(lines: &[&str], index: usize) -> usize {
    lines
        .iter()
        .enumerate()
        .skip(index + 1)
        .find(|(index, _)| is_lane_boundary(lines, *index))
        .map_or(lines.len(), |(index, _)| index)
}

pub(super) fn is_lane_boundary(lines: &[&str], index: usize) -> bool {
    let line = metadata_key(trimmed_value(lines[index]));
    "pr:|pull request:|review response:|maintainer reassignment:"
        .split('|')
        .any(|marker| line.starts_with(marker))
        || is_ownership_boundary(lines[index])
}

pub(super) fn is_ownership_boundary(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, _)| {
        "owner|child owner|lane owner|lane ownership|owner decision|ownership|pr ownership|pull request ownership"
            .split('|')
            .any(|boundary| metadata_key(key) == boundary)
    })
}

pub(super) fn is_legacy_ownership_boundary(line: &str) -> bool {
    let key = line
        .split_once(':')
        .map_or("", |(key, _)| metadata_key(key));
    "owner decision|ownership|lane ownership|pr ownership|pull request ownership"
        .split('|')
        .any(|boundary| key == boundary)
        || field_value(line, "owner").is_some_and(is_parent_owned_value)
}

pub(super) fn owner_at<'a>(
    evidence: &'a ClassificationEvidence<'_>,
    index: usize,
) -> Option<&'a str> {
    evidence
        .tables()
        .iter()
        .find(|table| table.start == index && table.canonical)
        .map(|table| table.owner.as_str())
}

pub(super) fn child_table_owns_handoff_pr(
    evidence: &ClassificationEvidence<'_>,
    pr_index: usize,
) -> bool {
    classification_owner_before(evidence, pr_index).is_some_and(is_child_delegation_owner_decision)
}

pub(super) fn table_ownership_boundary(
    evidence: &ClassificationEvidence<'_>,
    index: usize,
) -> bool {
    is_ownership_boundary(evidence.lines()[index])
        && classification_owner_before(evidence, index).is_some()
}

pub(super) fn child_table_ownership_boundary(
    evidence: &ClassificationEvidence<'_>,
    index: usize,
) -> bool {
    table_ownership_boundary(evidence, index)
        && evidence.lines()[index]
            .split_once(':')
            .is_some_and(|(key, value)| {
                let key = metadata_key(key);
                is_child_delegation_owner_decision(value)
                    || (key == "child owner"
                        && !value.is_empty()
                        && !value.starts_with("external/human-owned")
                        && !is_parent_owned_value(value)
                        && !value.starts_with("not ")
                        && !value.starts_with("without ")
                        && !matches!(value, "no" | "none" | "false" | "missing" | "absent"))
            })
}

pub(super) fn classification_owner_before<'a>(
    evidence: &'a ClassificationEvidence<'_>,
    index: usize,
) -> Option<&'a str> {
    let complete = applicable_canonical_tables(evidence, index);
    (complete.len() == 1).then(|| complete[0].owner.as_str())
}

pub(super) fn has_multiple_canonical_tables_before(
    evidence: &ClassificationEvidence<'_>,
    index: usize,
) -> bool {
    applicable_canonical_tables(evidence, index).len() > 1
}

fn applicable_canonical_tables<'a>(
    evidence: &'a ClassificationEvidence<'_>,
    index: usize,
) -> Vec<&'a ClassificationTable> {
    let lines = evidence.lines();
    let lane_start = current_lane_start(lines, index);
    evidence
        .tables()
        .iter()
        .filter(|table| {
            table.canonical
                && table.end < index
                && (handoff(table, lines, index, true)
                    || ((table.start >= lane_start
                        || (table.end < lane_start
                            && lines[table.end + 1..lane_start]
                                .iter()
                                .all(|line| line.is_empty())))
                        && !handoff(table, lines, index, false)))
        })
        .collect()
}

pub(super) fn handoff(
    table: &ClassificationTable,
    lines: &[&str],
    at: usize,
    separated: bool,
) -> bool {
    let metadata = &lines[table.end + 1..at];
    metadata
        .first()
        .is_some_and(|line| line.is_empty() == separated)
        && metadata.iter().all(|line| {
            line.is_empty()
                || line.split_once(':').is_some_and(|(key, _)| {
                    matches!(
                        metadata_key(key),
                        "issue" | "branch" | "worktree path" | "pr" | "pull request"
                    )
                })
        })
}

pub(super) fn candidate_requires_guard(
    evidence: &ClassificationEvidence<'_>,
    index: usize,
) -> bool {
    let lines = evidence.lines();
    let multiple = has_multiple_canonical_tables_before(evidence, index);
    evidence.tables().iter().any(|table| {
        let parent_child_transition = is_parent_owned_value(&table.owner)
            && (table.end + 1..index).any(|line| {
                is_ownership_boundary(lines[line])
                    && lines[line]
                        .split_once(':')
                        .is_some_and(|(_, value)| is_child_delegation_owner_decision(value))
            });
        table.start != index
            && (multiple
                || parent_child_transition
                || (!table.canonical
                    && (table.end >= index
                        || handoff(table, lines, index, true)
                        || (table.end + 1..index).all(|line| {
                            !is_lane_boundary(lines, line)
                                && !evidence.tables().iter().any(|table| table.start == line)
                        })))
                || (table.canonical
                    && ((table.end < index && handoff(table, lines, index, false))
                        || (table.start > index
                            && (index + 1..table.start)
                                .all(|line| !is_lane_boundary(lines, line))))))
    })
}
