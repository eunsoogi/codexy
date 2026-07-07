use super::child_lane_active_thread_count::{active_child_thread_count, key_words};
use super::child_lane_active_thread_evidence::ThreadOwner;

pub(super) const MAX_ACTIVE_CHILD_CODEX_THREADS: u64 = 5;

pub(super) fn active_child_thread_count_records(evidence: &str) -> Vec<ActiveCount> {
    let mut records = Vec::new();
    let mut freed_capacity_owner = None;
    for (line_number, line) in evidence.lines().enumerate() {
        let (count_records, trailing_freed_capacity) =
            active_count_records_for_line(line, line_number, freed_capacity_owner.clone());
        if count_records.is_empty() {
            if child_thread_freed_capacity(line) {
                freed_capacity_owner = Some(ThreadOwner::from_line(line));
            }
        } else {
            records.extend(count_records);
            freed_capacity_owner = trailing_freed_capacity;
        }
    }
    records
}

fn active_count_records_for_line(
    line: &str,
    line_number: usize,
    freed_capacity_owner: Option<ThreadOwner>,
) -> (Vec<ActiveCount>, Option<ThreadOwner>) {
    let mut records = Vec::new();
    let mut freed_capacity_owner = freed_capacity_owner;
    for segment in line.split(';').flat_map(|segment| segment.split(". ")) {
        let count_records = active_count_records_for_segment(
            line,
            segment,
            line_number,
            freed_capacity_owner.clone(),
        );
        if count_records.is_empty() {
            if child_thread_freed_capacity(segment) {
                freed_capacity_owner = Some(ThreadOwner::from_line(segment));
            }
        } else {
            records.extend(count_records);
            freed_capacity_owner = None;
        }
    }
    (records, freed_capacity_owner)
}

fn active_count_records_for_segment(
    line: &str,
    segment: &str,
    line_number: usize,
    freed_capacity_owner: Option<ThreadOwner>,
) -> Vec<ActiveCount> {
    let mut records = Vec::new();
    let mut freed_capacity_owner = freed_capacity_owner;
    for segment in split_count_comma_clauses(segment) {
        let Some(count) = active_child_thread_count(segment) else {
            if child_thread_freed_capacity(segment) {
                freed_capacity_owner = Some(ThreadOwner::from_line(segment));
            }
            continue;
        };
        records.push(ActiveCount {
            count,
            kind: count_kind(segment),
            line_number,
            segment_number: segment_offset(line, segment),
            freed_capacity: freed_capacity_owner.is_some(),
            freed_capacity_owner: freed_capacity_owner.clone(),
            owner: ThreadOwner::from_line(segment),
            thread_ids: thread_ids(segment),
        });
        freed_capacity_owner = None;
    }
    records
}
fn segment_offset(line: &str, segment: &str) -> usize {
    segment.as_ptr() as usize - line.as_ptr() as usize
}
fn split_count_comma_clauses(segment: &str) -> Vec<&str> {
    let lower = segment.to_ascii_lowercase();
    let mut clauses = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find(", ") {
        let marker_start = cursor + relative;
        let next_start = marker_start + ", ".len();
        if starts_count_clause(lower[next_start..].trim_start()) {
            clauses.push(&segment[start..marker_start]);
            start = next_start;
        }
        cursor = next_start;
    }
    clauses.push(&segment[start..]);
    clauses
}
fn starts_count_clause(clause: &str) -> bool {
    let words = key_words(clause);
    words.iter().any(|word| word == "child")
        && words
            .iter()
            .any(|word| matches!(word.as_str(), "thread" | "threads"))
        && (words.iter().any(|word| word == "active") || words.iter().any(|word| word == "waiting"))
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
        if record.freed_capacity {
            latest_active = None;
            latest_waiting = None;
        }
        match &record.kind {
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
    pub(super) segment_number: usize,
    pub(super) freed_capacity: bool,
    pub(super) freed_capacity_owner: Option<ThreadOwner>,
    kind: CountKind,
}

impl ActiveCount {
    pub(super) fn is_waiting(&self) -> bool {
        matches!(self.kind, CountKind::Waiting)
    }

    pub(super) fn replacement_counts_old_owner(&self, owner: &ThreadOwner) -> bool {
        self.freed_capacity
            && self
                .freed_capacity_owner
                .as_ref()
                .is_some_and(|freed_owner| thread_owner_matches(freed_owner, owner))
            && self.matches_owner(owner)
    }

    fn matches_owner(&self, owner: &ThreadOwner) -> bool {
        if let Some(owner_thread) = owner.thread_id.as_deref() {
            if !self.thread_ids.is_empty() {
                return self
                    .thread_ids
                    .iter()
                    .any(|thread_id| thread_id == owner_thread);
            }
            if let Some(record_thread) = self.owner.thread_id.as_deref() {
                return record_thread == owner_thread;
            }
        }
        !owner.issue_ids.is_empty()
            && self
                .owner
                .issue_ids
                .iter()
                .any(|id| owner.issue_ids.contains(id))
    }
}

enum CountKind {
    Active,
    Waiting,
}

fn count_kind(line: &str) -> CountKind {
    let words = key_words(line.split_once(':').map_or("", |(key, _)| key));
    if words.iter().any(|word| word == "waiting") && !words.iter().any(|word| word == "active") {
        CountKind::Waiting
    } else {
        CountKind::Active
    }
}

fn thread_ids(line: &str) -> Vec<String> {
    line.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '-' || character == '#')
    })
    .filter(|token| {
        token
            .strip_prefix("thread-")
            .is_some_and(|rest| !rest.is_empty())
            || is_non_prefixed_codex_thread_id(token)
    })
    .map(str::to_owned)
    .collect()
}

fn is_non_prefixed_codex_thread_id(token: &str) -> bool {
    !token.starts_with('#')
        && !token.starts_with("thread-")
        && token.len() >= 4
        && token
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
        && token.chars().any(|character| character.is_ascii_digit())
        && token
            .chars()
            .any(|character| character.is_ascii_alphabetic())
}

fn thread_owner_matches(found: &ThreadOwner, expected: &ThreadOwner) -> bool {
    if let (Some(found_thread), Some(expected_thread)) =
        (found.thread_id.as_deref(), expected.thread_id.as_deref())
    {
        return found_thread == expected_thread;
    }
    !expected.issue_ids.is_empty()
        && found
            .issue_ids
            .iter()
            .any(|id| expected.issue_ids.contains(id))
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
