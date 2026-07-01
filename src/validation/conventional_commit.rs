pub(super) fn check_pr_title(title: &str) -> Vec<String> {
    if is_conventional_subject(title) {
        Vec::new()
    } else {
        vec!["PR title must use Conventional Commit style".to_string()]
    }
}

pub(super) fn check_merge_subject(subject: &str, expected_pr: Option<u64>) -> Vec<String> {
    let subject = subject_without_expected_pr_suffix(subject, expected_pr);
    if is_conventional_subject(subject) {
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
    subject.strip_suffix(&expected_suffix).unwrap_or(subject)
}

fn is_conventional_subject(subject: &str) -> bool {
    let Some((prefix, summary)) = subject.split_once(": ") else {
        return false;
    };
    !summary.trim().is_empty() && is_conventional_prefix(prefix)
}

fn is_conventional_prefix(prefix: &str) -> bool {
    let prefix = prefix.strip_suffix('!').unwrap_or(prefix);
    let Some((commit_type, scope)) = prefix.split_once('(') else {
        return is_commit_type(prefix);
    };
    let Some(scope) = scope.strip_suffix(')') else {
        return false;
    };
    is_commit_type(commit_type) && is_scope(scope)
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
