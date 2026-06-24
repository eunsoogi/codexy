use serde_json::Value;

pub(super) fn matches_current_duplicate_state_target(text: &str, pr_state: &Value) -> bool {
    let pr_refs = referenced_numbers(text, &["pr #", "pull request #"]);
    let issue_refs = referenced_numbers(text, &["issue #"]);
    if pr_refs.is_empty() && issue_refs.is_empty() {
        return true;
    }

    let current_pr = pr_state.get("number").and_then(Value::as_u64);
    let current_issues: Vec<u64> = current_issue_values(pr_state)
        .iter()
        .filter_map(|issue| issue.get("number").and_then(Value::as_u64))
        .collect();

    let pr_match = !pr_refs.is_empty()
        && current_pr.is_some()
        && pr_refs.iter().all(|number| Some(*number) == current_pr);
    let issue_match = !issue_refs.is_empty()
        && !current_issues.is_empty()
        && issue_refs
            .iter()
            .all(|number| current_issues.contains(number));

    (pr_refs.is_empty() || pr_match) && (issue_refs.is_empty() || issue_match)
}

fn current_issue_values(pr_state: &Value) -> Vec<&Value> {
    let Some(issues) = pr_state.get("closingIssuesReferences") else {
        return Vec::new();
    };
    issues
        .as_array()
        .or_else(|| issues.get("nodes").and_then(Value::as_array))
        .into_iter()
        .flatten()
        .collect()
}

fn referenced_numbers(text: &str, prefixes: &[&str]) -> Vec<u64> {
    let mut numbers = Vec::new();
    for prefix in prefixes {
        let mut rest = text;
        while let Some(index) = rest.find(prefix) {
            let after_prefix = &rest[index + prefix.len()..];
            let digits: String = after_prefix
                .chars()
                .take_while(|character| character.is_ascii_digit())
                .collect();
            if let Ok(number) = digits.parse::<u64>() {
                numbers.push(number);
            }
            rest = after_prefix;
        }
    }
    numbers
}
