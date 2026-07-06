use super::child_lane_active_thread_count::{active_child_thread_count, key_words};
use super::child_lane_active_thread_evidence::ThreadOwner;

pub(super) const MAX_ACTIVE_CHILD_CODEX_THREADS: u64 = 5;

pub(super) fn active_child_thread_count_records(evidence: &str) -> Vec<ActiveCount> {
    let mut records = Vec::new();
    let mut freed_capacity = false;
    for (line_number, line) in evidence.lines().enumerate() {
        if let Some(count) = active_child_thread_count(line) {
            records.push(ActiveCount {
                count,
                kind: count_kind(line),
                line_number,
                freed_capacity,
                owner: ThreadOwner::from_line(line),
                thread_ids: thread_ids(line),
            });
            freed_capacity = false;
        } else if child_thread_freed_capacity(line) {
            freed_capacity = true;
        }
    }
    records
}

pub(super) fn active_child_thread_count_errors(active_counts: &[ActiveCount]) -> Vec<String> {
    let mut errors = active_counts
        .iter()
        .filter(|record| record.count > MAX_ACTIVE_CHILD_CODEX_THREADS)
        .map(|record| active_count_error(record.count))
        .collect::<Vec<_>>();
    let mut latest_active = None;
    let mut latest_waiting = None;
    for record in active_counts {
        match record.kind {
            CountKind::Active => latest_active = Some(record.count),
            CountKind::Waiting => latest_waiting = Some(record.count),
        }
        if let (Some(active), Some(waiting)) = (latest_active, latest_waiting) {
            let total = active.saturating_add(waiting);
            if total > MAX_ACTIVE_CHILD_CODEX_THREADS {
                errors.push(active_count_error(total));
            }
        }
    }
    errors
}

fn active_count_error(count: u64) -> String {
    format!(
        "orchestration evidence reports {count} active child Codex threads; keep at most five active child Codex threads before creating or resuming more"
    )
}

pub(super) struct ActiveCount {
    pub(super) count: u64,
    pub(super) owner: ThreadOwner,
    pub(super) thread_ids: Vec<String>,
    pub(super) line_number: usize,
    pub(super) freed_capacity: bool,
    kind: CountKind,
}

#[derive(Clone, Copy)]
enum CountKind {
    Active,
    Waiting,
}

fn count_kind(line: &str) -> CountKind {
    line.split_once(':')
        .map(|(key, _)| key_words(key))
        .filter(|words| words.iter().any(|word| word == "waiting"))
        .map_or(CountKind::Active, |_| CountKind::Waiting)
}

fn thread_ids(line: &str) -> Vec<String> {
    line.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '-' || character == '#')
    })
    .filter(|token| {
        token
            .strip_prefix("thread-")
            .is_some_and(|rest| !rest.is_empty())
    })
    .map(str::to_owned)
    .collect()
}

fn child_thread_freed_capacity(line: &str) -> bool {
    let words = key_words(line);
    words.iter().any(|word| word == "child")
        && words
            .iter()
            .any(|word| matches!(word.as_str(), "thread" | "threads"))
        && ["finished", "stopped", "removed"]
            .into_iter()
            .any(|marker| words.iter().any(|word| word == marker))
        && !["not", "no", "inactive"]
            .into_iter()
            .any(|marker| words.iter().any(|word| word == marker))
}
