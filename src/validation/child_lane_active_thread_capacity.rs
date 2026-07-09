use super::child_lane_active_thread_count_records::*;
use super::child_lane_active_thread_evidence::ThreadOwner;
use super::child_lane_active_thread_operations::ThreadOperation;
pub(super) fn active_capacity_errors(
    operations: &[ThreadOperation],
    active_counts: &[ActiveCount],
    existing_owners: &[Option<ThreadOwner>],
) -> Vec<String> {
    let mut errors = Vec::new();
    let mut previous_operation_position = None;
    let mut projected_count: Option<u64> = None;
    for (operation, existing_owner) in operations.iter().zip(existing_owners) {
        let mut counted_replacement = false;
        let records =
            fresh_counts_before_operation(active_counts, previous_operation_position, operation);
        if !records.is_empty() {
            counted_replacement = existing_owner.as_ref().is_some_and(|owner| {
                operation.replaces_existing_owner
                    && current_replacement_count_record(&records)
                        .is_some_and(|record| record.replacement_counts_old_owner(owner))
            });
            if let Some(record_count) = projected_count_from_records(&records) {
                projected_count = Some(match projected_count {
                    Some(_) if records.iter().any(|record| record.freed_capacity) => record_count,
                    Some(projected) => projected.max(record_count),
                    None => record_count,
                });
            } else {
                errors.push("new or resumed child Codex thread operations require evidence of the active child Codex thread count before the operation".to_owned());
            }
        } else if projected_count.is_none() {
            errors.push("new or resumed child Codex thread operations require evidence of the active child Codex thread count before the operation".to_owned());
        }
        if !continues_existing_owner(existing_owner.as_ref(), operation) && !counted_replacement {
            projected_count = Some(projected_count.unwrap_or(0).saturating_add(1));
        }
        if projected_count.is_some_and(|count| count > MAX_ACTIVE_CHILD_CODEX_THREADS) {
            errors.push("new or resumed child Codex thread operation would exceed five active child Codex threads".to_owned());
        }
        previous_operation_position = Some((operation.line_number, operation.segment_number));
    }
    errors
}
fn current_replacement_count_record<'a>(records: &[&'a ActiveCount]) -> Option<&'a ActiveCount> {
    records
        .iter()
        .rev()
        .copied()
        .find(|record| matches!(record.kind, CountKind::Active | CountKind::Total))
}
pub(super) fn continues_existing_owner(
    existing_owner: Option<&ThreadOwner>,
    operation: &ThreadOperation,
) -> bool {
    existing_owner
        .filter(|_| operation.reuses_existing_owner)
        .is_some_and(|existing_owner| {
            if let Some(operation_thread) = operation.owner.thread_id.as_deref() {
                return existing_owner.thread_id.as_deref() == Some(operation_thread);
            }
            !operation.owner.issue_ids.is_empty()
                && existing_owner
                    .issue_ids
                    .iter()
                    .any(|id| operation.owner.issue_ids.contains(id))
        })
}
fn fresh_counts_before_operation<'a>(
    active_counts: &'a [ActiveCount],
    previous_operation_position: Option<(usize, usize)>,
    operation: &ThreadOperation,
) -> Vec<&'a ActiveCount> {
    active_counts
        .iter()
        .filter(|record| {
            let record_position = (record.line_number, record.segment_number);
            record_position < (operation.line_number, operation.segment_number)
                && previous_operation_position.is_none_or(|position| record_position > position)
        })
        .collect()
}
fn projected_count_from_records(records: &[&ActiveCount]) -> Option<u64> {
    let mut latest = (None, None, None);
    for record in records {
        if record.freed_capacity {
            latest = (None, None, None);
        }
        match record.kind {
            CountKind::Total => latest = (None, None, Some(record.count)),
            CountKind::Waiting => latest.1 = Some(record.count),
            CountKind::Active => latest.0 = Some(record.count),
        }
        if let Some(active) = latest.0 {
            latest.2 = Some(active.saturating_add(latest.1.unwrap_or(0_u64)));
        }
    }
    latest.2
}
