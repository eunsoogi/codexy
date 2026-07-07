use super::child_lane_active_thread_count_records::{ActiveCount, MAX_ACTIVE_CHILD_CODEX_THREADS};
use super::child_lane_active_thread_evidence::ThreadOwner;

pub(super) fn child_thread_operations(evidence: &str) -> Vec<ThreadOperation> {
    evidence
        .lines()
        .enumerate()
        .flat_map(|(line_number, line)| {
            operation_segments(line).filter_map(move |segment| {
                (is_child_thread_operation_line(segment) && !has_negated_operation_claim(segment))
                    .then(|| ThreadOperation {
                        line_number,
                        segment_number: segment_offset(line, segment),
                        reuses_existing_owner: is_reuse_operation_line(segment),
                        replaces_existing_owner: normalized_operation_line(segment)
                            .contains("replacement child thread"),
                        owner: ThreadOwner::from_line(segment),
                    })
            })
        })
        .collect()
}
fn segment_offset(line: &str, segment: &str) -> usize {
    segment.as_ptr() as usize - line.as_ptr() as usize
}
fn operation_segments(line: &str) -> impl Iterator<Item = &str> {
    line.split(';')
        .flat_map(|line| line.split(". "))
        .flat_map(|line| split_operation_clauses(line, ", "))
        .flat_map(|line| line.split(", then "))
        .flat_map(|line| line.split(" then "))
        .flat_map(|line| split_operation_clauses(line, " and "))
}
fn split_operation_clauses<'a>(segment: &'a str, separator: &str) -> Vec<&'a str> {
    let lower = normalized_operation_line(segment);
    let mut clauses = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find(separator) {
        let marker_start = cursor + relative;
        let next_start = marker_start + separator.len();
        if starts_operation_clause(lower[next_start..].trim_start()) {
            clauses.push(&segment[start..marker_start]);
            start = next_start;
        }
        cursor = next_start;
    }
    clauses.push(&segment[start..]);
    clauses
}
fn starts_operation_clause(clause: &str) -> bool {
    let clause = clause
        .split_once(':')
        .filter(|(label, _)| label.contains("thread") || label.contains("operation"))
        .map_or(clause, |(_, rest)| rest.trim_start());
    operation_markers()
        .chain(["create_thread", "fork_thread", "send_message_to_thread"])
        .any(|marker| clause.starts_with(marker))
        || ["called", "invoked", "executed", "ran", "used"]
            .into_iter()
            .any(|verb| {
                ["create_thread", "fork_thread", "send_message_to_thread"]
                    .into_iter()
                    .any(|tool| clause.starts_with(&format!("{verb} {tool}")))
            })
}
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
        let count_bound = previous_operation_position
            .filter(|position| position != &(operation.line_number, operation.segment_number));
        let records = fresh_counts_before_operation(active_counts, count_bound, operation);
        if !records.is_empty() {
            counted_replacement = existing_owner.as_ref().is_some_and(|owner| {
                operation.replaces_existing_owner
                    && records
                        .iter()
                        .any(|record| record.replacement_counts_old_owner(owner))
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
        } else {
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
pub(super) fn continues_existing_owner(
    existing_owner: Option<&ThreadOwner>,
    operation: &ThreadOperation,
) -> bool {
    existing_owner
        .filter(|_| operation.reuses_existing_owner)
        .is_some_and(|existing_owner| {
            let existing_thread = existing_owner.thread_id.as_deref();
            existing_thread.is_some() && existing_thread == operation.owner.thread_id.as_deref()
        })
}

pub(super) struct ThreadOperation {
    pub(super) line_number: usize,
    pub(super) segment_number: usize,
    pub(super) owner: ThreadOwner,
    reuses_existing_owner: bool,
    replaces_existing_owner: bool,
}
fn is_child_thread_operation_line(line: &str) -> bool {
    let line = normalized_operation_line(line);
    line.contains("child thread")
        && (operation_markers().any(|marker| line.contains(marker))
            || has_passive_created_thread_id(&line))
        || ["create_thread", "fork_thread", "send_message_to_thread"]
            .into_iter()
            .any(|tool| is_thread_tool_invocation(&line, tool))
}
fn normalized_operation_line(line: &str) -> String {
    line.to_ascii_lowercase()
        .replace("child-thread", "child thread")
        .replace("child codex app thread", "child thread")
        .replace("child codex thread", "child thread")
        .replace("created a new child thread", "created child thread")
        .replace("created new child thread", "created child thread")
        .replace("created a child thread", "created child thread")
        .replace("forked a child thread", "forked child thread")
        .replace("forked the child thread", "forked child thread")
        .replace("started a child thread", "started child thread")
        .replace("started the child thread", "started child thread")
        .replace("continued the child thread", "continued child thread")
        .replace("continued a child thread", "continued child thread")
        .replace("resumed the child thread", "resumed child thread")
        .replace("resumed a child thread", "resumed child thread")
}
fn operation_markers() -> impl Iterator<Item = &'static str> {
    "child thread created:|created child thread|created a replacement child thread|created replacement child thread|continued child thread|forked child thread|forked a child thread|resumed child thread|started child thread|started a child thread".split('|')
}
fn has_passive_created_thread_id(line: &str) -> bool {
    line.find("child thread").is_some_and(|index| {
        let rest = &line[index + "child thread".len()..];
        rest.split_whitespace()
            .next()
            .is_some_and(|token| token.starts_with("thread-"))
            && (rest.contains(" created") || rest.contains(" was created"))
    })
}
fn is_thread_tool_invocation(line: &str, tool: &str) -> bool {
    if has_negated_thread_tool_reference(line, tool) {
        return false;
    }
    line.match_indices(tool)
        .any(|(index, _)| line[index + tool.len()..].trim_start().starts_with('('))
        || (["called", "invoked", "executed", "ran", "used"]
            .into_iter()
            .any(|word| line.contains(word))
            && line.contains(tool)
            && !["tool search", "discovered", "available thread tool"]
                .into_iter()
                .any(|marker| line.contains(marker)))
}
fn has_negated_thread_tool_reference(line: &str, tool: &str) -> bool {
    format!("{tool} was not used|{tool} wasn't used|{tool} is not used|{tool} not used|did not use {tool}|didn't use {tool}|do not use {tool}|must not use {tool}|not using {tool}|without using {tool}")
        .split('|')
    .any(|marker| line.contains(&marker))
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
    let mut latest_active = None;
    let mut latest_waiting = None;
    for record in records {
        if record.freed_capacity {
            latest_active = None;
            latest_waiting = None;
        }
        if record.is_waiting() {
            latest_waiting = Some(record.count);
        } else {
            latest_active = Some(record.count);
        }
    }
    latest_active.map(|active| active.saturating_add(latest_waiting.unwrap_or(0_u64)))
}

fn is_reuse_operation_line(line: &str) -> bool {
    let line = normalized_operation_line(line);
    "thread resume:|thread continuation:|continued child thread|resumed child thread|send_message_to_thread"
        .split('|')
        .any(|marker| line.contains(marker))
}

fn has_negated_operation_claim(line: &str) -> bool {
    let line = normalized_operation_line(line);
    let operation_position = |clause: &str| {
        operation_markers()
            .chain(["create_thread", "fork_thread", "send_message_to_thread"])
            .filter_map(|marker| clause.find(marker))
            .min()
    };
    let negation_position = |clause: &str| {
        "did not call|did not continue|did not create|did not resume|didn't call|didn't continue|didn't create|didn't resume|do not call|do not continue|do not create|do not resume|must not call|must not continue|must not create|must not resume|not call|not continue|not create|not resume|no child thread created|no child thread continued|no child thread resumed|without calling|without continuing|without creating|without resuming"
            .split('|')
            .filter_map(|marker| clause.find(marker))
            .min()
    };
    let mut has_negated_operation = false;
    let mut has_unnegated_operation = false;
    for clause in line.split(';').flat_map(|clause| clause.split(". ")) {
        if let Some(operation) = operation_position(clause) {
            match negation_position(clause) {
                Some(negation) if negation <= operation => has_negated_operation = true,
                _ => has_unnegated_operation = true,
            }
        }
    }
    has_negated_operation && !has_unnegated_operation
}
