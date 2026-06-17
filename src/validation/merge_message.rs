pub(super) fn check(expected_issue: u64, message: &str) -> Vec<String> {
    if has_unique_final_closing_reference(expected_issue, message) {
        Vec::new()
    } else {
        vec![format!(
            "merge commit message must contain exactly one closing reference, and the final closing line must be exactly: Fixes #{expected_issue}"
        )]
    }
}

fn has_unique_final_closing_reference(expected_issue: u64, message: &str) -> bool {
    let expected_line = format!("Fixes #{expected_issue}");
    let non_empty_lines = message
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let closing_reference_count = non_empty_lines
        .iter()
        .map(|line| closing_reference_count(line))
        .sum::<usize>();
    closing_reference_count == 1 && non_empty_lines.last() == Some(&expected_line.as_str())
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
