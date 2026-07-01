pub(super) fn check(
    expected_issue: Option<u64>,
    expected_pr: Option<u64>,
    message: &str,
) -> Vec<String> {
    let mut errors = Vec::new();
    let subject = message.lines().next().unwrap_or_default();
    errors.extend(super::conventional_commit::check_merge_subject(
        subject,
        expected_pr,
    ));
    if let Some(expected_pr) = expected_pr {
        if !has_expected_pr_suffix(expected_pr, message) {
            errors.push(format!(
                "merge commit subject must end with the expected PR suffix: (#{expected_pr})"
            ));
        }
    }
    if let Some(expected_issue) = expected_issue {
        if !has_unique_final_closing_reference(expected_issue, message) {
            errors.push(format!(
                "merge commit message must contain exactly one closing reference, and the final closing line must be exactly: Fixes #{expected_issue}"
            ));
        }
    } else if expected_pr.is_some() && has_closing_reference(message) {
        errors.push("merge commit message must not contain closing references".to_string());
    }
    errors
}

fn has_expected_pr_suffix(expected_pr: u64, message: &str) -> bool {
    let expected_suffix = format!("(#{expected_pr})");
    message
        .lines()
        .next()
        .is_some_and(|line| line.ends_with(&expected_suffix))
}

fn has_unique_final_closing_reference(expected_issue: u64, message: &str) -> bool {
    let expected_line = format!("Fixes #{expected_issue}");
    let non_empty_lines = message
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let closing_reference_count =
        closing_reference_count_for_lines(non_empty_lines.iter().copied());
    closing_reference_count == 1 && non_empty_lines.last() == Some(&expected_line.as_str())
}

fn has_closing_reference(message: &str) -> bool {
    closing_reference_count_for_lines(message.lines()) > 0
}

fn closing_reference_count_for_lines<'a>(lines: impl Iterator<Item = &'a str>) -> usize {
    lines.map(closing_reference_count).sum()
}

fn closing_reference_count(line: &str) -> usize {
    let tokens = line.split_ascii_whitespace().collect::<Vec<_>>();
    let mut count = 0;
    for (index, token) in tokens.iter().enumerate() {
        let keyword = token.strip_suffix(':').unwrap_or(token);
        if !is_closing_keyword(keyword) {
            continue;
        }
        for candidate in tokens.iter().skip(index + 1) {
            if is_closing_issue_reference(candidate) {
                count += 1;
                continue;
            }
            break;
        }
    }
    count
}

fn is_closing_keyword(keyword: &str) -> bool {
    matches!(
        keyword.to_ascii_lowercase().as_str(),
        "close"
            | "closes"
            | "closed"
            | "fix"
            | "fixes"
            | "fixed"
            | "resolve"
            | "resolves"
            | "resolved"
    )
}

fn is_closing_issue_reference(candidate: &str) -> bool {
    let candidate = candidate.trim_matches(|character: char| matches!(character, ',' | '.'));
    if let Some(reference) = candidate.strip_prefix('#') {
        return is_issue_number(reference);
    }
    let Some((owner_repo, issue)) = candidate.rsplit_once('#') else {
        return false;
    };
    is_owner_repo_reference(owner_repo) && is_issue_number(issue)
}

fn is_issue_number(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|character| character.is_ascii_digit())
}

fn is_owner_repo_reference(value: &str) -> bool {
    let Some((owner, repo)) = value.split_once('/') else {
        return false;
    };
    !owner.is_empty()
        && !repo.is_empty()
        && [owner, repo].iter().all(|part| {
            part.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
            })
        })
}
