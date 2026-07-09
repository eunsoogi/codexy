use super::child_lane_active_thread_capacity::{active_capacity_errors, continues_existing_owner};
use super::child_lane_active_thread_count_records::{
    active_child_thread_count_errors, active_child_thread_count_records,
};
use super::child_lane_active_thread_evidence::{
    OwnerLookup, ThreadOwner, issue_ids, matching_owner_lookup_before, thread_id,
};
use super::child_lane_active_thread_operations::{ThreadOperation, child_thread_operations};

pub(super) fn check(evidence: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let operations = child_thread_operations(evidence);
    let has_child_thread_operation = !operations.is_empty();
    let active_counts = active_child_thread_count_records(evidence);
    let mut previous_operation: Option<&ThreadOperation> = None;
    let owner_lookups = operations
        .iter()
        .map(|operation| {
            let lookup_bound = previous_operation.and_then(|previous| {
                shares_owner_id(&previous.owner, &operation.owner)
                    .then_some((previous.line_number, previous.segment_number))
            });
            let lookup = matching_owner_lookup_before(
                evidence,
                &operation.owner,
                operation.line_number,
                operation.segment_number,
                lookup_bound,
            );
            previous_operation = Some(operation);
            lookup
        })
        .collect::<Vec<_>>();
    let existing_owners = owner_lookups
        .iter()
        .map(|lookup| match lookup {
            Some(OwnerLookup::Found(owner)) => Some(owner.clone()),
            Some(OwnerLookup::NotFound(_)) | None => None,
        })
        .collect::<Vec<_>>();
    errors.extend(active_child_thread_count_errors(&active_counts));
    if has_child_thread_operation && active_counts.is_empty() {
        errors.push("new or resumed child Codex thread operations require evidence of the active child Codex thread count before the operation".to_owned());
    }
    errors.extend(active_capacity_errors(
        &operations,
        &active_counts,
        &existing_owners,
    ));
    if has_child_thread_operation && owner_lookups.iter().any(Option::is_none) {
        errors.push("new child Codex thread creation requires evidence that orchestration checked for an existing issue/PR owner thread and reused it when present before the operation".to_owned());
    }
    if has_child_thread_operation
        && operations
            .iter()
            .zip(&owner_lookups)
            .enumerate()
            .any(|(index, (operation, lookup))| {
                let Some(OwnerLookup::Found(existing_owner)) = lookup else {
                    return false;
                };
                let previous_operation_position = index.checked_sub(1).and_then(|previous| {
                    shares_owner_id(&operations[previous].owner, &operation.owner).then_some((
                        operations[previous].line_number,
                        operations[previous].segment_number,
                    ))
                });
                !continues_existing_owner(Some(existing_owner), operation)
                    && !has_matching_old_owner_disposition_before(
                        evidence,
                        Some(existing_owner),
                        previous_operation_position,
                        (operation.line_number, operation.segment_number),
                    )
            })
    {
        errors.push("replacement child Codex thread creation requires evidence that the old owner was stopped, unusable, or explicitly superseded".to_owned());
    }
    errors
}

fn shares_issue_id(left: &ThreadOwner, right: &ThreadOwner) -> bool {
    !left.issue_ids.is_empty()
        && left
            .issue_ids
            .iter()
            .any(|issue_id| right.issue_ids.contains(issue_id))
}

fn shares_owner_id(left: &ThreadOwner, right: &ThreadOwner) -> bool {
    shares_issue_id(left, right)
        || left
            .thread_id
            .as_deref()
            .is_some_and(|thread_id| right.thread_id.as_deref() == Some(thread_id))
}

fn has_matching_old_owner_disposition_before(
    evidence: &str,
    existing_owner: Option<&ThreadOwner>,
    previous_operation_position: Option<(usize, usize)>,
    operation_position: (usize, usize),
) -> bool {
    evidence.lines().enumerate().any(|(line_number, line)| {
        let Some(disposition_offset) = matching_old_owner_disposition_offset(line, existing_owner)
        else {
            return false;
        };
        let position = (line_number, disposition_offset);
        position < operation_position
            && previous_operation_position.is_none_or(|previous| position > previous)
    })
}

fn matching_old_owner_disposition_offset(
    line: &str,
    existing_owner: Option<&ThreadOwner>,
) -> Option<usize> {
    disposition_segments(line).find_map(|segment| {
        let normalized_segment = segment.to_ascii_lowercase();
        if !(normalized_segment.contains("old owner")
            || normalized_segment.contains("existing owner thread"))
            || !disposition_matches_owner(segment, existing_owner)
        {
            return None;
        }
        accepted_disposition_claim_offset(segment)
            .map(|offset| segment_offset(line, segment) + offset)
    })
}

fn disposition_segments(line: &str) -> impl Iterator<Item = &str> {
    line.split(';')
        .flat_map(|segment| segment.split(". "))
        .flat_map(split_disposition_comma_clauses)
}

fn split_disposition_comma_clauses(segment: &str) -> Vec<&str> {
    let lower = segment.to_ascii_lowercase();
    let mut clauses = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find(", ") {
        let marker_start = cursor + relative;
        let next_start = marker_start + ", ".len();
        let next_clause = lower[next_start..].trim_start();
        if next_clause.starts_with("old owner")
            || next_clause.starts_with("existing owner thread")
            || next_clause.starts_with("disposition:")
            || next_clause.starts_with("thread creation:")
        {
            clauses.push(&segment[start..marker_start]);
            start = next_start;
        }
        cursor = next_start;
    }
    clauses.push(&segment[start..]);
    clauses
}

fn segment_offset(line: &str, segment: &str) -> usize {
    segment.as_ptr() as usize - line.as_ptr() as usize
}

fn disposition_matches_owner(line: &str, existing_owner: Option<&ThreadOwner>) -> bool {
    let Some(existing_owner) = existing_owner else {
        return true;
    };
    let line_thread = thread_id(line);
    let line_issues = issue_ids(line);
    if let (Some(line_thread), Some(owner_thread)) =
        (line_thread.as_deref(), existing_owner.thread_id.as_deref())
    {
        return line_thread == owner_thread;
    }
    !existing_owner.issue_ids.is_empty()
        && existing_owner.issue_ids.iter().any(|owner_issue| {
            line_issues
                .iter()
                .any(|line_issue| line_issue == owner_issue)
        })
}

fn accepted_disposition_claim_offset(line: &str) -> Option<usize> {
    let words = line_words_with_offsets(line);
    words
        .iter()
        .enumerate()
        .find_map(|(index, (offset, word))| {
            (matches!(word.as_str(), "stopped" | "unusable" | "superseded")
                && !disposition_claim_is_negated(&words, index))
            .then_some(*offset)
        })
}

fn disposition_claim_is_negated(words: &[(usize, String)], index: usize) -> bool {
    words
        .iter()
        .take(index)
        .rev()
        .take_while(|(_, word)| word != "but")
        .take(5)
        .any(|(_, word)| matches!(word.as_str(), "not" | "never" | "wasnt" | "wasn"))
}

fn line_words_with_offsets(line: &str) -> Vec<(usize, String)> {
    let mut words = Vec::new();
    let mut start = None;
    for (index, character) in line.char_indices() {
        if character.is_ascii_alphanumeric() {
            start.get_or_insert(index);
        } else if let Some(word_start) = start.take() {
            words.push((word_start, line[word_start..index].to_ascii_lowercase()));
        }
    }
    if let Some(word_start) = start {
        words.push((word_start, line[word_start..].to_ascii_lowercase()));
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_owner_disposition_matching_normalizes_case() {
        let evidence = "\
old owner disposition: thread-148 was STOPPED as UNUSABLE and explicitly SUPERSEDED for issue #269.
Thread creation: created replacement child thread thread-269 for issue #269.";
        let owner = ThreadOwner {
            thread_id: Some("thread-148".to_owned()),
            issue_ids: vec!["#269".to_owned()],
        };

        assert!(has_matching_old_owner_disposition_before(
            evidence,
            Some(&owner),
            None,
            (1, 0),
        ));
    }
}
