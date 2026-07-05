use super::child_lane_active_thread_evidence::{ThreadOwner, issue_id, thread_id};

pub(super) const MAX_ACTIVE_CHILD_CODEX_THREADS: u64 = 5;

pub(super) fn active_child_thread_count_records(evidence: &str) -> Vec<ActiveCount> {
    let mut records = Vec::new();
    let mut freed_capacity = false;
    for (line_number, line) in evidence.lines().enumerate() {
        if let Some(count) = active_child_thread_count(line) {
            records.push(ActiveCount {
                count,
                line_number,
                freed_capacity,
            });
            freed_capacity = false;
        } else if child_thread_freed_capacity(line) {
            freed_capacity = true;
        }
    }
    records
}

pub(super) fn child_thread_operations(evidence: &str) -> Vec<ThreadOperation> {
    evidence
        .lines()
        .enumerate()
        .filter_map(|line| {
            let (line_number, line) = line;
            (is_child_thread_operation_line(line) && !has_negated_operation_claim(line)).then(
                || ThreadOperation {
                    line_number,
                    reuses_existing_owner: is_reuse_operation_line(line),
                    owner: ThreadOwner {
                        thread_id: thread_id(line),
                        issue_id: issue_id(line),
                    },
                },
            )
        })
        .collect()
}

pub(super) fn active_capacity_errors(
    operations: &[ThreadOperation],
    active_counts: &[ActiveCount],
    existing_owners: &[Option<ThreadOwner>],
) -> Vec<String> {
    let mut errors = Vec::new();
    let mut previous_operation_line = None;
    let mut projected_count: Option<u64> = None;
    for (operation, existing_owner) in operations.iter().zip(existing_owners) {
        if let Some(record) =
            fresh_count_before_operation(active_counts, previous_operation_line, operation)
        {
            projected_count = Some(projected_active_count(projected_count, record));
        } else {
            errors.push("new or resumed child Codex thread operations require evidence of the active child Codex thread count before the operation".to_owned());
        }

        if !continues_existing_owner(existing_owner.as_ref(), operation) {
            projected_count = Some(projected_count.unwrap_or(0).saturating_add(1));
        }
        if projected_count.is_some_and(|count| count > MAX_ACTIVE_CHILD_CODEX_THREADS) {
            errors.push("new or resumed child Codex thread operation would exceed five active child Codex threads".to_owned());
        }
        previous_operation_line = Some(operation.line_number);
    }
    errors
}

pub(super) fn continues_existing_owner(
    existing_owner: Option<&ThreadOwner>,
    operation: &ThreadOperation,
) -> bool {
    let Some(existing_owner) = existing_owner else {
        return false;
    };
    if !operation.reuses_existing_owner {
        return false;
    }
    existing_owner
        .thread_id
        .as_deref()
        .zip(operation.owner.thread_id.as_deref())
        .is_some_and(|(existing, operation)| existing == operation)
}

pub(super) struct ThreadOperation {
    pub(super) line_number: usize,
    pub(super) owner: ThreadOwner,
    reuses_existing_owner: bool,
}

pub(super) struct ActiveCount {
    pub(super) count: u64,
    line_number: usize,
    freed_capacity: bool,
}

fn active_child_thread_count(line: &str) -> Option<u64> {
    let (key, value) = line.split_once(':')?;
    let key_words = key_words(key);
    if !has_active_child_thread_key(&key_words) {
        return None;
    }
    first_unsigned_integer(value)
}

fn key_words(key: &str) -> Vec<String> {
    key.to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

fn has_active_child_thread_key(words: &[String]) -> bool {
    words.iter().any(|word| word == "active")
        && words.iter().any(|word| word == "child")
        && words
            .iter()
            .any(|word| word == "thread" || word == "threads")
        && !words.iter().any(|word| word == "inactive")
        && !words
            .windows(2)
            .any(|window| window[0] == "non" && window[1] == "active")
        && !words.iter().any(|word| word == "subagent")
        && !words.iter().any(|word| word == "specialist")
}

fn first_unsigned_integer(value: &str) -> Option<u64> {
    value
        .split(|character: char| !character.is_ascii_digit())
        .find(|part| !part.is_empty())
        .and_then(|part| part.parse().ok())
}

fn child_thread_freed_capacity(line: &str) -> bool {
    let words = key_words(line);
    words.iter().any(|word| word == "child")
        && words.iter().any(|word| word == "thread")
        && words.iter().any(|word| word == "active")
        && (words.iter().any(|word| word == "finished")
            || words.iter().any(|word| word == "stopped")
            || words.iter().any(|word| word == "removed"))
        && !words.iter().any(|word| word == "not")
        && !words.iter().any(|word| word == "inactive")
}

fn is_child_thread_operation_line(line: &str) -> bool {
    let has_child_thread = line.contains("child thread") || line.contains("child codex thread");
    if has_child_thread
        && [
            "thread creation:",
            "thread resume:",
            "thread continuation:",
            "created child thread",
            "created replacement child thread",
            "continued child thread",
            "forked child thread",
            "resumed child thread",
            "started child thread",
        ]
        .into_iter()
        .any(|marker| line.contains(marker))
    {
        return true;
    }
    ["create_thread", "fork_thread", "send_message_to_thread"]
        .into_iter()
        .any(|tool| {
            line.contains(tool)
                && [
                    "called",
                    "calling",
                    "continued",
                    "created",
                    "invoked",
                    "invoking",
                    "resumed",
                    "started",
                ]
                .into_iter()
                .any(|marker| line.contains(marker))
        })
}

fn fresh_count_before_operation<'a>(
    active_counts: &'a [ActiveCount],
    previous_operation_line: Option<usize>,
    operation: &ThreadOperation,
) -> Option<&'a ActiveCount> {
    active_counts.iter().rev().find(|record| {
        record.line_number < operation.line_number
            && previous_operation_line.is_none_or(|line_number| record.line_number > line_number)
    })
}

fn projected_active_count(projected_count: Option<u64>, fresh_count: &ActiveCount) -> u64 {
    match projected_count {
        Some(_) if fresh_count.freed_capacity => fresh_count.count,
        Some(projected) => projected.max(fresh_count.count),
        None => fresh_count.count,
    }
}

fn is_reuse_operation_line(line: &str) -> bool {
    [
        "thread resume:",
        "thread continuation:",
        "continued child thread",
        "resumed child thread",
        "send_message_to_thread",
    ]
    .into_iter()
    .any(|marker| line.contains(marker))
}

fn has_negated_operation_claim(line: &str) -> bool {
    [
        "did not continue",
        "did not create",
        "did not resume",
        "didn't continue",
        "didn't create",
        "didn't resume",
        "do not continue",
        "do not create",
        "do not resume",
        "must not continue",
        "must not create",
        "must not resume",
        "not continue",
        "not create",
        "not resume",
        "no child thread created",
        "no child thread continued",
        "no child thread resumed",
        "without continuing",
        "without creating",
        "without resuming",
    ]
    .into_iter()
    .any(|marker| line.contains(marker))
}
