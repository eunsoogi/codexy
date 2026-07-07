use serde_json::Value;

pub(super) fn issue_label_errors(issues: Vec<&Value>) -> Vec<String> {
    issues
        .into_iter()
        .filter_map(|issue| {
            let number = issue
                .get("number")
                .and_then(Value::as_u64)
                .map_or_else(|| "<unknown>".to_owned(), |number| format!("#{number}"));
            label_names(issue.get("labels"))
                .is_empty()
                .then(|| format!("issue {number} labels missing label application evidence"))
        })
        .collect()
}

pub(super) fn label_names(labels: Option<&Value>) -> Vec<String> {
    match labels {
        Some(Value::Array(items)) => items.iter().filter_map(label_name).collect(),
        Some(Value::Object(map)) => map
            .get("nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(label_name)
            .collect(),
        _ => Vec::new(),
    }
}

fn label_name(value: &Value) -> Option<String> {
    value
        .as_str()
        .or_else(|| value.get("name").and_then(Value::as_str))
        .map(str::to_owned)
}

pub(super) fn issue_nodes(issues: Option<&Value>) -> Vec<&Value> {
    match issues {
        Some(Value::Array(items)) => items.iter().collect(),
        Some(Value::Object(map)) => map
            .get("nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .collect(),
        _ => Vec::new(),
    }
}

pub(super) fn stacked_issue_evidence(pr_state: &Value) -> Option<Vec<&Value>> {
    let issue_number = closing_keyword_issue_number(pr_state.get("body").and_then(Value::as_str))?;
    let issues = issue_nodes(pr_state.get("linkedIssueReferences"))
        .into_iter()
        .filter(|issue| {
            issue
                .get("number")
                .and_then(Value::as_u64)
                .is_some_and(|number| number == issue_number)
                && issue_url_matches(issue, issue_number, pr_state)
        })
        .collect::<Vec<_>>();
    (!issues.is_empty()).then_some(issues)
}

fn issue_url_matches(issue: &Value, issue_number: u64, pr_state: &Value) -> bool {
    let Some(repository) = repository_name_with_owner(pr_state) else {
        return false;
    };
    let expected_url = format!("https://github.com/{repository}/issues/{issue_number}");
    issue
        .get("url")
        .and_then(Value::as_str)
        .map(|url| url.trim_end_matches('/').to_ascii_lowercase())
        .is_some_and(|url| url == expected_url)
}

fn closing_keyword_issue_number(body: Option<&str>) -> Option<u64> {
    let line = body?.lines().rev().find(|line| !line.trim().is_empty())?;
    let rest = ["Fixes #", "Closes #", "Resolves #"]
        .into_iter()
        .find_map(|keyword| line.strip_prefix(keyword))?;
    let digits = rest
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    if digits.len() != rest.len() {
        return None;
    }
    (!digits.is_empty()).then(|| digits.parse().ok()).flatten()
}

pub(super) fn repository_label_taxonomy(pr_state: &Value) -> Option<Vec<String>> {
    let mut found_empty_taxonomy = false;
    let mut labels = pr_state
        .get("repositoryLabels")
        .into_iter()
        .chain(pr_state.pointer("/repository/labels"));
    for labels in labels.by_ref().filter(|labels| {
        matches!(labels, Value::Array(_)) || matches!(labels.get("nodes"), Some(Value::Array(_)))
    }) {
        let names = label_names(Some(labels));
        if !names.is_empty() {
            return Some(names);
        }
        found_empty_taxonomy = true;
    }
    found_empty_taxonomy.then(Vec::new)
}

pub(super) fn is_open_pr(pr_state: &Value) -> bool {
    matches!(
        pr_state.get("state").and_then(Value::as_str),
        Some(state) if state.eq_ignore_ascii_case("OPEN")
    )
}

pub(super) fn is_stacked_pr(pr_state: &Value) -> bool {
    let Some(base) = pr_state
        .get("baseRefName")
        .and_then(Value::as_str)
        .filter(|base| !base.trim().is_empty())
    else {
        return false;
    };
    let default_branch = pr_state
        .pointer("/defaultBranchRef/name")
        .or_else(|| pr_state.pointer("/repository/defaultBranchRef/name"))
        .and_then(Value::as_str)
        .or_else(|| pr_state.get("defaultBranchName").and_then(Value::as_str))
        .unwrap_or("main");
    base != default_branch
}

pub(super) fn is_codexy_lane(pr_state: &Value) -> bool {
    string_field(pr_state, &["repository", "nameWithOwner", "headRepository"])
        .iter()
        .any(|value| value == "eunsoogi/codexy")
        || string_field(pr_state, &["url"]).iter().any(|value| {
            value.contains("github.com/eunsoogi/codexy/")
                || value.ends_with("github.com/eunsoogi/codexy")
        })
}

fn string_field(value: &Value, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .filter_map(|key| value.get(*key).and_then(Value::as_str))
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

fn repository_name_with_owner(pr_state: &Value) -> Option<String> {
    pr_state
        .get("repository")
        .and_then(|repository| {
            repository
                .as_str()
                .or_else(|| repository.get("nameWithOwner").and_then(Value::as_str))
        })
        .or_else(|| pr_state.get("nameWithOwner").and_then(Value::as_str))
        .or_else(|| pr_state.get("headRepository").and_then(Value::as_str))
        .and_then(normalize_repository_name)
        .or_else(|| {
            pr_state
                .get("url")
                .and_then(Value::as_str)
                .and_then(repository_name_from_github_url)
        })
}

fn normalize_repository_name(value: &str) -> Option<String> {
    let repository = value.trim().trim_matches('/').to_ascii_lowercase();
    let mut parts = repository.split('/');
    let owner = parts.next()?.trim();
    let name = parts.next()?.trim();
    (parts.next().is_none() && !owner.is_empty() && !name.is_empty())
        .then(|| format!("{owner}/{name}"))
}

fn repository_name_from_github_url(value: &str) -> Option<String> {
    let rest = value
        .trim()
        .strip_prefix("https://github.com/")
        .or_else(|| value.trim().strip_prefix("http://github.com/"))?;
    let mut parts = rest.split('/');
    let owner = parts.next()?.trim();
    let name = parts.next()?.trim();
    (!owner.is_empty() && !name.is_empty()).then(|| {
        format!(
            "{}/{}",
            owner.to_ascii_lowercase(),
            name.to_ascii_lowercase()
        )
    })
}
