use serde_json::Value;

pub(super) fn matches_current_duplicate_state_target(text: &str, pr_state: &Value) -> bool {
    let pr_refs = referenced_numbers(text, &["pr", "pull request"]);
    let issue_refs = referenced_numbers(text, &["issue"]);
    if pr_refs.is_empty() && issue_refs.is_empty() {
        return true;
    }

    let current_pr = pr_state.get("number").and_then(Value::as_u64);
    let current_issues = current_issue_numbers(pr_state);

    let pr_match = !pr_refs.is_empty()
        && current_pr.is_some()
        && pr_refs.iter().all(|number| Some(*number) == current_pr);
    let issue_match = !issue_refs.is_empty()
        && current_issues.as_ref().is_none_or(|issues| {
            !issues.is_empty() && issue_refs.iter().all(|number| issues.contains(number))
        });

    (pr_refs.is_empty() || pr_match) && (issue_refs.is_empty() || issue_match)
}

fn current_issue_numbers(pr_state: &Value) -> Option<Vec<u64>> {
    Some(
        current_issue_values(pr_state)?
            .iter()
            .filter_map(|issue| issue.get("number").and_then(Value::as_u64))
            .collect(),
    )
}

fn current_issue_values(pr_state: &Value) -> Option<Vec<&Value>> {
    let Some(issues) = pr_state.get("closingIssuesReferences") else {
        return None;
    };
    Some(
        issues
            .as_array()
            .or_else(|| issues.get("nodes").and_then(Value::as_array))
            .into_iter()
            .flatten()
            .collect(),
    )
}

fn referenced_numbers(text: &str, prefixes: &[&str]) -> Vec<u64> {
    let mut numbers = Vec::new();
    for prefix in prefixes {
        let mut start = 0;
        while let Some(relative_index) = text[start..].find(prefix) {
            let index = start + relative_index;
            let after_prefix = &text[index + prefix.len()..];
            start = index + prefix.len();
            if !has_reference_prefix_boundary(text, index) {
                continue;
            }
            let Some(after_marker) = reference_number_start(after_prefix) else {
                continue;
            };
            let digits: String = after_prefix[after_marker..]
                .chars()
                .take_while(|character| character.is_ascii_digit())
                .collect();
            if let Ok(number) = digits.parse::<u64>() {
                numbers.push(number);
            }
        }
    }
    numbers
}

fn has_reference_prefix_boundary(text: &str, index: usize) -> bool {
    text[..index]
        .chars()
        .next_back()
        .is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn reference_number_start(text: &str) -> Option<usize> {
    let mut offset = text.len() - text.trim_start().len();
    let mut rest = &text[offset..];
    if let Some(after_hash) = rest.strip_prefix('#') {
        offset += 1;
        rest = after_hash;
    }
    let whitespace_after_hash = rest.len() - rest.trim_start().len();
    offset += whitespace_after_hash;
    rest = &rest[whitespace_after_hash..];
    rest.chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
        .then_some(offset)
}
