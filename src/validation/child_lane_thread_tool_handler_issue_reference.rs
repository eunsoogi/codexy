pub(super) fn has_issue_reference(clause: &str) -> bool {
    has_hash_issue_reference(clause)
        || has_github_issue_url(clause)
        || has_repository_qualified_issue_reference(clause)
}

fn has_hash_issue_reference(clause: &str) -> bool {
    clause.match_indices('#').any(|(hash_index, _)| {
        let token_start = clause[..hash_index]
            .rfind(|character: char| character.is_whitespace())
            .map_or(0, |index| index + 1);
        if token_start != hash_index {
            return false;
        }
        let issue_tail = &clause[hash_index + 1..];
        let digit_end = issue_tail
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(issue_tail.len());
        digit_end > 0 && is_bare_issue_boundary(&issue_tail[digit_end..])
    })
}

fn has_github_issue_url(clause: &str) -> bool {
    clause.split_whitespace().any(|word| {
        let trimmed = word.trim_matches(|character: char| {
            !character.is_ascii_alphanumeric() && !":/.#_-".contains(character)
        });
        if !trimmed.starts_with("https://github.com/") && !trimmed.starts_with("http://github.com/")
        {
            return false;
        }
        let Some((_, issue_tail)) = trimmed.rsplit_once("/issues/") else {
            return false;
        };
        let digit_end = issue_tail
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(issue_tail.len());
        digit_end > 0 && is_issue_url_boundary(&issue_tail[digit_end..])
    })
}

fn has_repository_qualified_issue_reference(clause: &str) -> bool {
    clause.split_whitespace().any(|word| {
        let trimmed = word.trim_matches(|character: char| {
            !character.is_ascii_alphanumeric() && !"/.#_-".contains(character)
        });
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return false;
        }
        let Some((repository, issue_tail)) = trimmed.rsplit_once('#') else {
            return false;
        };
        let Some((owner, repo)) = repository.rsplit_once('/') else {
            return false;
        };
        if !is_repository_reference_segment(owner) || !is_repository_reference_segment(repo) {
            return false;
        }
        let digit_end = issue_tail
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(issue_tail.len());
        digit_end > 0 && is_issue_url_boundary(&issue_tail[digit_end..])
    })
}

fn is_repository_reference_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

fn is_bare_issue_boundary(suffix: &str) -> bool {
    suffix.is_empty()
        || suffix
            .chars()
            .next()
            .is_some_and(|character| character.is_whitespace())
        || suffix.starts_with('/')
        || suffix
            .chars()
            .all(|character| matches!(character, '.' | ',' | ')' | ']' | '}' | '>' | '"' | '\''))
}

fn is_issue_url_boundary(suffix: &str) -> bool {
    suffix.is_empty()
        || suffix == "/"
        || suffix
            .chars()
            .all(|character| matches!(character, '.' | ',' | ')' | ']' | '}' | '>' | '"' | '\''))
}
