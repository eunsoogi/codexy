pub(super) fn has_issue_reference(clause: &str) -> bool {
    has_hash_issue_reference(clause)
        || has_github_issue_url(clause)
        || has_repository_qualified_issue_reference(clause)
}

fn has_hash_issue_reference(clause: &str) -> bool {
    clause.match_indices('#').any(|(hash_index, _)| {
        if !is_bare_issue_start(&clause[..hash_index]) {
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
        let Some(url_start) = github_url_start(trimmed) else {
            return false;
        };
        let url = &trimmed[url_start..];
        let Some((repository_url, issue_tail)) = url.rsplit_once("/issues/") else {
            return false;
        };
        if !has_github_owner_and_repo(repository_url) {
            return false;
        }
        let digit_end = issue_tail
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(issue_tail.len());
        digit_end > 0 && is_issue_url_boundary(&issue_tail[digit_end..])
    })
}

fn github_url_start(candidate: &str) -> Option<usize> {
    candidate
        .find("https://github.com/")
        .or_else(|| candidate.find("http://github.com/"))
}

fn has_github_owner_and_repo(repository_url: &str) -> bool {
    let Some(repository) = repository_url
        .strip_prefix("https://github.com/")
        .or_else(|| repository_url.strip_prefix("http://github.com/"))
    else {
        return false;
    };
    let Some((owner, repo)) = repository.rsplit_once('/') else {
        return false;
    };
    is_repository_reference_segment(owner) && is_repository_reference_segment(repo)
}

fn has_repository_qualified_issue_reference(clause: &str) -> bool {
    clause.split_whitespace().any(|word| {
        let candidate = word.trim_matches(|character: char| {
            !character.is_ascii_alphanumeric() && !"/.#_-".contains(character)
        });
        if candidate.starts_with("http://") || candidate.starts_with("https://") {
            return false;
        }
        let trimmed = candidate
            .rsplit_once(':')
            .map_or(candidate, |(_, issue_ref)| issue_ref);
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
        && segment
            .chars()
            .any(|character| character.is_ascii_alphanumeric())
        && segment.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

fn is_bare_issue_start(prefix: &str) -> bool {
    let token_prefix = prefix
        .rsplit_once(|character: char| character.is_whitespace())
        .map_or(prefix, |(_, token)| token);
    if token_prefix.contains('/') {
        return false;
    }

    match prefix.chars().next_back() {
        None => true,
        Some(character) => !character.is_ascii_alphanumeric() && !matches!(character, '/' | '#'),
    }
}

fn is_bare_issue_boundary(suffix: &str) -> bool {
    suffix.is_empty()
        || suffix
            .chars()
            .next()
            .is_some_and(|character| character.is_whitespace())
        || suffix.starts_with('/')
        || suffix.chars().next().is_some_and(is_bare_issue_delimiter)
        || suffix
            .chars()
            .all(|character| matches!(character, '.' | ',' | ')' | ']' | '}' | '>' | '"' | '\''))
}

fn is_bare_issue_delimiter(character: char) -> bool {
    !character.is_ascii_alphanumeric() && !matches!(character, '#' | '/')
}

fn is_issue_url_boundary(suffix: &str) -> bool {
    suffix.is_empty()
        || suffix == "/"
        || suffix
            .chars()
            .all(|character| matches!(character, '.' | ',' | ')' | ']' | '}' | '>' | '"' | '\''))
}
