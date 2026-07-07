use super::child_lane_active_thread_capacity::{
    active_capacity_errors, child_thread_operations, continues_existing_owner,
};
use super::child_lane_active_thread_count_records::{
    active_child_thread_count_errors, active_child_thread_count_records,
};
use super::child_lane_active_thread_evidence::{
    OwnerLookup, ThreadOwner, issue_ids, matching_owner_lookup_before, thread_id,
};

pub(super) fn check(evidence: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let operations = child_thread_operations(evidence);
    let has_child_thread_operation = !operations.is_empty();
    let active_counts = active_child_thread_count_records(evidence);
    let mut previous_operation_position = None;
    let owner_lookups = operations
        .iter()
        .map(|operation| {
            let lookup_bound = previous_operation_position
                .filter(|position| position != &(operation.line_number, operation.segment_number));
            let lookup = matching_owner_lookup_before(
                evidence,
                &operation.owner,
                operation.line_number,
                operation.segment_number,
                lookup_bound,
            );
            previous_operation_position = Some((operation.line_number, operation.segment_number));
            lookup
        })
        .collect::<Vec<_>>();
    let existing_owners = owner_lookups
        .iter()
        .map(|lookup| match lookup {
            Some(OwnerLookup::Found(owner)) => Some(owner.clone()),
            Some(OwnerLookup::NotFound) | None => None,
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
                let previous_operation_position = index.checked_sub(1).map(|previous| {
                    (
                        operations[previous].line_number,
                        operations[previous].segment_number,
                    )
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

fn has_matching_old_owner_disposition_before(
    evidence: &str,
    existing_owner: Option<&ThreadOwner>,
    previous_operation_position: Option<(usize, usize)>,
    operation_position: (usize, usize),
) -> bool {
    evidence.lines().enumerate().any(|(line_number, line)| {
        let normalized_line = line.to_ascii_lowercase();
        let position = (line_number, old_owner_disposition_offset(line));
        position < operation_position
            && previous_operation_position.is_none_or(|previous| position > previous)
            && (normalized_line.contains("old owner")
                || normalized_line.contains("existing owner thread"))
            && has_accepted_disposition_claim(line)
            && disposition_matches_owner(line, existing_owner)
    })
}

fn old_owner_disposition_offset(line: &str) -> usize {
    let line = line.to_ascii_lowercase();
    line.find("old owner")
        .or_else(|| line.find("existing owner thread"))
        .unwrap_or(0)
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
        && line_issues
            .iter()
            .any(|line_issue| existing_owner.issue_ids.iter().any(|id| id == line_issue))
}

fn has_accepted_disposition_claim(line: &str) -> bool {
    let words = line
        .to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    words.iter().enumerate().any(|(index, word)| {
        matches!(word.as_str(), "stopped" | "unusable" | "superseded")
            && !words
                .iter()
                .take(index)
                .rev()
                .take(3)
                .any(|word| matches!(word.as_str(), "not" | "never" | "wasnt" | "wasn"))
    })
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
