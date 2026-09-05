pub(super) fn check_pr_title(title: &str) -> Vec<String> {
    if is_conventional_subject(title, true) {
        Vec::new()
    } else {
        vec!["PR title must use Conventional Commit style".to_string()]
    }
}

pub(super) fn check_issue_title(title: &str) -> Vec<String> {
    if is_issue_category(title) {
        vec!["issue title must not use Conventional Commit style".to_string()]
    } else if !starts_with_ascii_uppercase(title) || has_invalid_title_character(title) {
        vec!["issue title must start with an uppercase descriptive title".to_string()]
    } else {
        Vec::new()
    }
}

pub(super) fn check_merge_subject(subject: &str, expected_pr: Option<u64>) -> Vec<String> {
    let subject = subject_without_expected_pr_suffix(subject, expected_pr);
    if is_conventional_subject(subject, expected_pr.is_some()) {
        Vec::new()
    } else {
        vec!["merge commit subject must use Conventional Commit style".to_string()]
    }
}

fn subject_without_expected_pr_suffix(subject: &str, expected_pr: Option<u64>) -> &str {
    let Some(expected_pr) = expected_pr else {
        return subject;
    };
    let expected_suffix = format!(" (#{expected_pr})");
    if let Some(subject) = subject.strip_suffix(&expected_suffix) {
        return subject;
    }
    subject
}

fn is_conventional_subject(subject: &str, reject_terminal_reference: bool) -> bool {
    if has_invalid_title_character(subject) {
        return false;
    }
    let Some((prefix, summary)) = subject.split_once(": ") else {
        return false;
    };
    !summary.trim().is_empty()
        && is_conventional_prefix(prefix)
        && (!reject_terminal_reference || !has_terminal_reference(summary))
}

fn is_conventional_prefix(prefix: &str) -> bool {
    let prefix = prefix.strip_suffix('!').unwrap_or(prefix);
    let Some((commit_type, scope)) = prefix.split_once('(') else {
        return false;
    };
    let Some(scope) = scope.strip_suffix(')') else {
        return false;
    };
    !scope.contains('(') && is_commit_type(commit_type) && is_scope(scope)
}

fn has_terminal_reference(summary: &str) -> bool {
    let mut candidate = summary.trim();
    while candidate.ends_with(['.', ',']) {
        candidate = candidate[..candidate.len() - 1].trim_end();
    }
    let tokens = candidate.split_ascii_whitespace().collect::<Vec<_>>();
    let Some(last) = tokens.last().copied() else {
        return false;
    };
    let last = last.trim_end_matches(['.', ',']);
    if is_hash_reference(last) {
        return true;
    }
    if let Some(inner) = last
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        return is_hash_reference(inner);
    }
    if tokens.len() > 1
        && matches!(
            tokens[tokens.len() - 2].to_ascii_lowercase().as_str(),
            "pr" | "issue"
        )
        && is_hash_reference(last)
    {
        return true;
    }
    if let Some((before, inner)) = candidate.rsplit_once('(') {
        if let Some(inner) = inner.strip_suffix(')') {
            let parts = inner.split_ascii_whitespace().collect::<Vec<_>>();
            let separated = before.is_empty()
                || before
                    .chars()
                    .last()
                    .is_some_and(|character| character.is_ascii_whitespace());
            if separated
                && ((parts.len() == 1 && is_hash_reference(parts[0]))
                    || (parts.len() == 2
                        && matches!(parts[0].to_ascii_lowercase().as_str(), "pr" | "issue")
                        && is_hash_reference(parts[1])))
            {
                return true;
            }
        }
    }
    false
}

fn is_hash_reference(value: &str) -> bool {
    value.strip_prefix('#').is_some_and(|number| {
        !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn is_issue_category(value: &str) -> bool {
    if value.starts_with('[') {
        if let Some(end) = value.find(']') {
            let inner = &value[1..end];
            if parse_category_prefix(inner).is_some_and(|(index, _, _)| index == inner.len()) {
                return true;
            }
        }
    }
    let Some((index, scoped, breaking)) = parse_category_prefix(value) else {
        return false;
    };
    let rest = &value[index..];
    if scoped || breaking {
        return rest.is_empty()
            || rest.starts_with(' ')
            || rest.starts_with('\t')
            || label_separator(rest);
    }
    if rest.is_empty()
        || rest
            .chars()
            .all(|character| [' ', '\t'].contains(&character))
    {
        return true;
    }
    let trimmed = rest.trim_start_matches(|character| [' ', '\t'].contains(&character));
    trimmed.starts_with(':') || trimmed.starts_with('：') || label_separator(trimmed)
}

fn parse_category_prefix(value: &str) -> Option<(usize, bool, bool)> {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() && is_type_character(bytes[index]) {
        index += 1;
    }
    if index == 0 || !is_commit_type(&value[..index].to_ascii_lowercase()) {
        return None;
    }
    let type_end = index;
    let mut next = type_end;
    while next < bytes.len() && is_ascii_space(bytes[next]) {
        next += 1;
    }
    let mut scoped = false;
    let mut index = type_end;
    if bytes.get(next) == Some(&b'(') {
        scoped = true;
        index = next + 1;
        while index < bytes.len() && is_ascii_space(bytes[index]) {
            index += 1;
        }
        let scope_start = index;
        while index < bytes.len() && is_scope_character(bytes[index]) {
            index += 1;
        }
        if scope_start == index {
            return None;
        }
        let scope = value[scope_start..index].to_ascii_lowercase();
        if !is_scope(&scope) {
            return None;
        }
        while index < bytes.len() && is_ascii_space(bytes[index]) {
            index += 1;
        }
        if bytes.get(index) != Some(&b')') {
            return None;
        }
        index += 1;
    }
    let mut breaking_start = index;
    while breaking_start < bytes.len() && is_ascii_space(bytes[breaking_start]) {
        breaking_start += 1;
    }
    let breaking = bytes.get(breaking_start) == Some(&b'!');
    if breaking {
        index = breaking_start + 1;
        while index < bytes.len() && is_ascii_space(bytes[index]) {
            index += 1;
        }
    }
    Some((index, scoped, breaking))
}

fn label_separator(value: &str) -> bool {
    let mut characters = value.chars();
    match characters.next() {
        Some(':' | '：') => true,
        Some('-' | '–' | '—') => characters
            .next()
            .is_none_or(|character| matches!(character, ' ' | '\t')),
        _ => false,
    }
}

fn starts_with_ascii_uppercase(value: &str) -> bool {
    value.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
}

fn has_invalid_title_character(value: &str) -> bool {
    value
        .chars()
        .any(|character| character.is_control() || matches!(character, '\u{2028}' | '\u{2029}'))
}

fn is_type_character(value: u8) -> bool {
    value.is_ascii_alphanumeric() || value == b'-'
}

fn is_scope_character(value: u8) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, b'-' | b'_' | b'/')
}

fn is_ascii_space(value: u8) -> bool {
    matches!(value, b' ' | b'\t')
}

fn is_commit_type(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

fn is_scope(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '-' | '_' | '/')
        })
}
