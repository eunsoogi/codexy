use super::child_lane_active_thread_owner_lookup_segments::{
    has_negated_owner_check_claim, owner_lookup_segments,
};
#[derive(Clone, Debug)]
pub(super) struct ThreadOwner {
    pub(super) thread_id: Option<String>,
    pub(super) issue_ids: Vec<String>,
}
impl ThreadOwner {
    pub(super) fn from_line(line: &str) -> Self {
        Self {
            thread_id: thread_id(line),
            issue_ids: issue_ids(line),
        }
    }
}
pub(super) enum OwnerLookup {
    Found(ThreadOwner),
    NotFound(Vec<String>),
}
pub(super) fn matching_owner_lookup_before(
    evidence: &str,
    operation_owner: &ThreadOwner,
    operation_line_number: usize,
    operation_segment_number: usize,
    previous_operation_position: Option<(usize, usize)>,
) -> Option<OwnerLookup> {
    let mut latest = None;
    let mut not_found_ids = Vec::new();
    for (line_number, line) in evidence.lines().enumerate() {
        if line_number > operation_line_number {
            continue;
        }
        if previous_operation_position.is_some_and(|previous| (line_number, usize::MAX) <= previous)
        {
            continue;
        }
        let lower_bound = previous_operation_position
            .filter(|(previous_line, _)| *previous_line == line_number)
            .map(|(_, previous_segment)| previous_segment);
        let upper_bound =
            (line_number == operation_line_number).then_some(operation_segment_number);
        if let Some(lookup) =
            owner_lookup_for_operation(line, operation_owner, lower_bound, upper_bound)
        {
            match lookup {
                OwnerLookup::Found(owner) => latest = Some(OwnerLookup::Found(owner)),
                OwnerLookup::NotFound(ids) => {
                    if !matches!(latest, Some(OwnerLookup::Found(_))) {
                        not_found_ids.extend(ids);
                        if !operation_owner.issue_ids.is_empty()
                            && operation_owner
                                .issue_ids
                                .iter()
                                .all(|id| not_found_ids.contains(id))
                        {
                            latest = Some(OwnerLookup::NotFound(not_found_ids.clone()));
                        }
                    }
                }
            }
        }
    }
    latest
}
pub(super) fn thread_id(line: &str) -> Option<String> {
    token_with_prefix(line, "thread-")
        .or_else(|| thread_id_argument(line))
        .or_else(|| non_prefixed_thread_id(line))
}
fn token_with_prefix(line: &str, prefix: &str) -> Option<String> {
    line.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '-' || character == '#')
    })
    .find(|token| {
        token
            .to_ascii_lowercase()
            .strip_prefix(prefix)
            .is_some_and(|rest| !rest.is_empty())
    })
    .map(str::to_ascii_lowercase)
}
fn thread_id_argument(line: &str) -> Option<String> {
    let (_, value) = line.split_once("thread_id")?;
    value
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '-' || character == '#')
        })
        .find(|token| is_codex_thread_id(token))
        .map(str::to_owned)
}
fn non_prefixed_thread_id(line: &str) -> Option<String> {
    let mut tokens = line.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '-' || character == '#')
    });
    while let Some(token) = tokens.next() {
        if token.eq_ignore_ascii_case("thread") {
            if let Some(next) = tokens.next() {
                if next.eq_ignore_ascii_case("id") {
                    if let Some(thread_id) = tokens.next().filter(|next| is_codex_thread_id(next)) {
                        return Some(thread_id.to_owned());
                    }
                } else if is_codex_thread_id(next) {
                    return Some(next.to_owned());
                }
            }
        }
    }
    None
}
fn is_codex_thread_id(token: &str) -> bool {
    !token.starts_with('#')
        && !token.starts_with("thread-")
        && token.len() >= 4
        && token
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
        && token.chars().any(|character| character.is_ascii_digit())
}
pub(super) fn issue_ids(line: &str) -> Vec<String> {
    let mut ids = issue_hash_tokens(line);
    ids.extend(
        ["issue", "pr"]
            .into_iter()
            .filter_map(|marker| number_after_marker(line, marker)),
    );
    ids
}
fn issue_hash_tokens(line: &str) -> Vec<String> {
    line.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '-' || character == '#')
    })
    .filter_map(|token| token.strip_prefix('#'))
    .filter(|number| {
        !number.is_empty() && number.chars().all(|character| character.is_ascii_digit())
    })
    .map(|number| format!("#{number}"))
    .collect()
}
fn number_after_marker(line: &str, marker: &str) -> Option<String> {
    let mut tokens =
        line.split(|character: char| !(character.is_ascii_alphanumeric() || character == '-'));
    while let Some(token) = tokens.next() {
        if token.eq_ignore_ascii_case(marker) {
            if let Some(number) = tokens
                .next()
                .and_then(|next| next.strip_prefix('#').or(Some(next)))
                .filter(|next| {
                    !next.is_empty() && next.chars().all(|character| character.is_ascii_digit())
                })
            {
                return Some(format!("#{number}"));
            }
        }
    }
    None
}
fn line_contains_existing_owner_found(line: &str) -> bool {
    let line = normalized_owner_lookup_line(line);
    line.contains("owner thread")
        && line.contains("found")
        && !line_contains_no_existing_owner_found(line)
}
fn normalized_owner_lookup_line(line: &str) -> String {
    line.to_ascii_lowercase().replace('-', " ")
}
fn line_contains_no_existing_owner_found(line: impl AsRef<str>) -> bool {
    let line = normalized_owner_lookup_line(line.as_ref());
    "no existing owner thread found|no existing issue owner thread found|no existing pr owner thread found|no existing issue/pr owner thread found|no existing issue or pr owner thread found|no existing owner thread was found|no existing issue owner thread was found|no existing pr owner thread was found|no existing issue/pr owner thread was found|no existing issue or pr owner thread was found|found no existing owner thread|found no existing issue owner thread|found no existing pr owner thread|found no existing issue/pr owner thread|found no existing issue or pr owner thread|existing owner thread not found|existing issue owner thread not found|existing pr owner thread not found|existing issue/pr owner thread not found|existing issue or pr owner thread not found|owner thread not found"
        .split('|')
        .any(|marker| line.contains(marker))
        || (line.contains("none found")
            && (line.contains("owner check") || line.contains("owner thread")))
}
fn owner_lookup(line: &str) -> Option<OwnerLookup> {
    if line_contains_existing_owner_found(line) {
        return Some(OwnerLookup::Found(ThreadOwner::from_line(line)));
    }
    line_contains_no_existing_owner_found(line).then(|| OwnerLookup::NotFound(issue_ids(line)))
}
fn owner_lookup_for_operation(
    line: &str,
    operation_owner: &ThreadOwner,
    lower_bound: Option<usize>,
    upper_bound: Option<usize>,
) -> Option<OwnerLookup> {
    let mut not_found_ids = Vec::new();
    let mut partial_found = false;
    let mut negated_lookup = false;
    for segment in owner_lookup_segments(line).into_iter().filter(|segment| {
        lower_bound.is_none_or(|bound| segment.position > bound)
            && upper_bound.is_none_or(|bound| segment.position < bound)
    }) {
        if has_negated_owner_check_claim(segment.text) && is_owner_lookup_context(segment.text) {
            negated_lookup = true;
            continue;
        }
        if negated_lookup || !lookup_matches_any_operation_id(segment.text, operation_owner) {
            continue;
        }
        match owner_lookup(segment.text) {
            Some(OwnerLookup::Found(owner))
                if lookup_matches_operation(segment.text, operation_owner) =>
            {
                return Some(OwnerLookup::Found(owner));
            }
            Some(OwnerLookup::Found(_)) => partial_found = true,
            Some(OwnerLookup::NotFound(ids)) => not_found_ids.extend(ids),
            None => {}
        }
    }
    (!partial_found && !not_found_ids.is_empty()).then_some(OwnerLookup::NotFound(not_found_ids))
}
fn is_owner_lookup_context(line: &str) -> bool {
    let line = normalized_owner_lookup_line(line);
    line.contains("owner check") || line.contains("owner thread")
}
fn lookup_matches_operation(line: &str, operation_owner: &ThreadOwner) -> bool {
    let line_issues = issue_ids(line);
    operation_owner
        .issue_ids
        .iter()
        .all(|id| line_issues.contains(id))
}
fn lookup_matches_any_operation_id(line: &str, operation_owner: &ThreadOwner) -> bool {
    if !operation_owner.issue_ids.is_empty() {
        let line_issues = issue_ids(line);
        return operation_owner.issue_ids.iter().any(|operation_issue| {
            line_issues
                .iter()
                .any(|line_issue| line_issue == operation_issue)
        });
    }
    let Some(operation_thread) = operation_owner.thread_id.as_deref() else {
        return false;
    };
    line_contains_existing_owner_found(line)
        && thread_id(line)
            .as_deref()
            .is_some_and(|line_thread| line_thread == operation_thread)
}
