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
    NotFound,
}

pub(super) fn matching_owner_lookup_before(
    evidence: &str,
    operation_owner: &ThreadOwner,
    operation_line_number: usize,
    previous_operation_line: Option<usize>,
) -> Option<OwnerLookup> {
    let mut latest = None;
    for (line_number, line) in evidence.lines().enumerate() {
        if line_number < operation_line_number
            && previous_operation_line.is_none_or(|previous| line_number > previous)
            && !has_negated_owner_check_claim(line)
        {
            if let Some(lookup) = owner_lookup_for_operation(line, operation_owner) {
                if matches!(
                    (&latest, &lookup),
                    (Some(OwnerLookup::Found(_)), OwnerLookup::NotFound)
                ) {
                    continue;
                }
                latest = Some(lookup);
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
            .strip_prefix(prefix)
            .is_some_and(|rest| !rest.is_empty())
    })
    .map(str::to_owned)
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
    if let Some(issue) = number_after_marker(line, "issue") {
        ids.push(issue);
    }
    if let Some(pr) = number_after_marker(line, "pr") {
        ids.push(pr);
    }
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
    let mut tokens = line.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '-' || character == '#')
    });
    while let Some(token) = tokens.next() {
        if token.eq_ignore_ascii_case(marker) {
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
    let line = normalized_owner_lookup_line(line);
    line.contains("owner thread")
        && line.contains("found")
        && !line_contains_no_existing_owner_found(line)
}

fn normalized_owner_lookup_line(line: &str) -> String {
    line.to_ascii_lowercase()
        .replace("owner-thread", "owner thread")
}

fn line_contains_no_existing_owner_found(line: impl AsRef<str>) -> bool {
    let line = normalized_owner_lookup_line(line.as_ref());
    line.contains("no existing owner thread found")
        || line.contains("no existing issue owner thread found")
        || line.contains("no existing pr owner thread found")
        || line.contains("no existing issue/pr owner thread found")
        || line.contains("no existing issue or pr owner thread found")
        || line.contains("no existing owner thread was found")
        || line.contains("no existing issue owner thread was found")
        || line.contains("no existing pr owner thread was found")
        || line.contains("no existing issue/pr owner thread was found")
        || line.contains("no existing issue or pr owner thread was found")
        || line.contains("found no existing owner thread")
        || line.contains("found no existing issue owner thread")
        || line.contains("found no existing pr owner thread")
        || line.contains("found no existing issue/pr owner thread")
        || line.contains("found no existing issue or pr owner thread")
        || line.contains("existing owner thread not found")
        || line.contains("existing issue owner thread not found")
        || line.contains("existing pr owner thread not found")
        || line.contains("existing issue/pr owner thread not found")
        || line.contains("existing issue or pr owner thread not found")
        || line.contains("owner thread not found")
        || (line.contains("none found")
            && (line.contains("owner check") || line.contains("owner thread")))
}

fn owner_lookup(line: &str) -> Option<OwnerLookup> {
    if line_contains_existing_owner_found(line) {
        return Some(OwnerLookup::Found(ThreadOwner::from_line(line)));
    }
    line_contains_no_existing_owner_found(line).then_some(OwnerLookup::NotFound)
}

fn owner_lookup_for_operation(line: &str, operation_owner: &ThreadOwner) -> Option<OwnerLookup> {
    let mut not_found = None;
    for segment in line
        .split(';')
        .map(str::trim)
        .filter(|segment| lookup_matches_operation(segment, operation_owner))
    {
        match owner_lookup(segment) {
            Some(OwnerLookup::Found(owner)) => return Some(OwnerLookup::Found(owner)),
            Some(OwnerLookup::NotFound) => not_found = Some(OwnerLookup::NotFound),
            None => {}
        }
    }
    not_found
}

fn lookup_matches_operation(line: &str, operation_owner: &ThreadOwner) -> bool {
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

fn has_negated_owner_check_claim(line: &str) -> bool {
    let line = normalized_owner_lookup_line(line);
    if [
        "not run",
        "not checked",
        "without checking",
        "no existing owner thread evidence",
        "no existing evidence",
        "without evidence",
        "no evidence",
        "missing evidence",
    ]
    .into_iter()
    .any(|marker| line.contains(marker))
    {
        return true;
    }
    ["not found", "none found", "without owner"]
        .into_iter()
        .any(|marker| line.contains(marker))
        && !line_contains_no_existing_owner_found(line)
}
