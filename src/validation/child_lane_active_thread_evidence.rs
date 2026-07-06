#[derive(Clone, Debug)]
pub(super) struct ThreadOwner {
    pub(super) thread_id: Option<String>,
    pub(super) issue_id: Option<String>,
}

pub(super) enum OwnerLookup {
    Found(ThreadOwner),
    NotFound,
}

pub(super) fn matching_owner_lookup_before(
    evidence: &str,
    operation_owner: &ThreadOwner,
    operation_line_number: usize,
) -> Option<OwnerLookup> {
    let mut latest = None;
    for (line_number, line) in evidence.lines().enumerate() {
        if line_number < operation_line_number
            && !has_negated_owner_check_claim(line)
            && lookup_matches_operation(line, operation_owner)
        {
            if let Some(lookup) = owner_lookup(line) {
                latest = Some(lookup);
            }
        }
    }
    latest
}

pub(super) fn thread_id(line: &str) -> Option<String> {
    token_with_prefix(line, "thread-").or_else(|| non_prefixed_thread_id(line))
}

pub(super) fn issue_id(line: &str) -> Option<String> {
    token_with_prefix(line, "#")
        .or_else(|| number_after_marker(line, "issue"))
        .or_else(|| number_after_marker(line, "pr"))
}

fn token_with_prefix(line: &str, prefix: &str) -> Option<String> {
    line.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '-' || character == '#')
    })
    .find(|token| {
        token
            .strip_prefix(prefix)
            .is_some_and(|rest| !rest.is_empty())
    })
    .map(str::to_owned)
}

fn non_prefixed_thread_id(line: &str) -> Option<String> {
    let mut tokens = line.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '-' || character == '#')
    });
    while let Some(token) = tokens.next() {
        if token.eq_ignore_ascii_case("thread") {
            if let Some(thread_id) = tokens.next().filter(|next| is_codex_thread_id(next)) {
                return Some(thread_id.to_owned());
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

fn number_after_marker(line: &str, marker: &str) -> Option<String> {
    let mut tokens = line.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '-' || character == '#')
    });
    while let Some(token) = tokens.next() {
        if token == marker {
            if let Some(number) = tokens
                .next()
                .and_then(|next| next.strip_prefix('#').or(Some(next)))
                .filter(|next| next.chars().all(|character| character.is_ascii_digit()))
            {
                return Some(format!("#{number}"));
            }
        }
    }
    None
}

fn line_contains_existing_owner_found(line: &str) -> bool {
    line.contains("owner thread")
        && line.contains("found")
        && !line_contains_no_existing_owner_found(line)
}

fn line_contains_no_existing_owner_found(line: &str) -> bool {
    line.contains("no existing owner thread found")
        || line.contains("no existing issue owner thread found")
        || line.contains("no existing pr owner thread found")
}

fn owner_lookup(line: &str) -> Option<OwnerLookup> {
    if line_contains_existing_owner_found(line) {
        return Some(OwnerLookup::Found(ThreadOwner {
            thread_id: thread_id(line),
            issue_id: issue_id(line),
        }));
    }
    line_contains_no_existing_owner_found(line).then_some(OwnerLookup::NotFound)
}

fn lookup_matches_operation(line: &str, operation_owner: &ThreadOwner) -> bool {
    if let Some(operation_issue) = operation_owner.issue_id.as_deref() {
        return issue_id(line)
            .as_deref()
            .is_some_and(|line_issue| line_issue == operation_issue);
    }
    let Some(operation_thread) = operation_owner.thread_id.as_deref() else {
        return false;
    };
    line_contains_existing_owner_found(line)
        && thread_id(line)
            .as_deref()
            .is_some_and(|line_thread| line_thread == operation_thread)
}

fn has_negated_owner_check_claim(line: &str) -> bool {
    [
        "not run",
        "not checked",
        "not found",
        "none found",
        "without checking",
        "without owner",
        "no existing owner thread evidence",
    ]
    .into_iter()
    .any(|marker| line.contains(marker))
        && !line_contains_no_existing_owner_found(line)
}
