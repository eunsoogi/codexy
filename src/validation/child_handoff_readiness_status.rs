use serde_json::Value;

const STATUS_FIELDS: [&str; 5] = [
    "worktreeStatus",
    "localStatus",
    "gitStatus",
    "gitStatusShort",
    "statusShort",
];

pub(super) fn status_fields(pr_state: &Value) -> impl Iterator<Item = String> + '_ {
    STATUS_FIELDS
        .into_iter()
        .filter_map(|field| pr_state.get(field))
        .filter_map(status_lines)
        .flatten()
}

pub(super) fn dirty_status(lines: &[String]) -> Option<String> {
    lines
        .iter()
        .any(|line| is_dirty_status_line(line))
        .then(|| lines.join("; "))
}

pub(super) fn branch_status_not_pushed(lines: &[String]) -> Option<&str> {
    lines
        .iter()
        .find(|line| {
            line.contains("[ahead ") || line.contains("[behind ") || line.contains("[gone]")
        })
        .map(String::as_str)
}

pub(super) fn branch_status_not_pr_branch<'a>(
    lines: &'a [String],
    pr_state: &Value,
) -> Option<&'a str> {
    let has_branch_header = lines.iter().any(|line| line.starts_with("## "));
    if !has_branch_header {
        return Some("current branch status evidence is missing");
    }
    let Some(head) = string_field(pr_state, "headRefName") else {
        return lines
            .iter()
            .find(|line| line.starts_with("## "))
            .map(String::as_str);
    };
    let prefix = format!("## {head}...");
    lines
        .iter()
        .find(|line| line.starts_with("## ") && !line.starts_with(&prefix))
        .map(String::as_str)
}

pub(super) fn pr_branch_statuses(pr_state: &Value) -> Vec<String> {
    let Some(head) = string_field(pr_state, "headRefName") else {
        return Vec::new();
    };
    let prefix = format!("## {head}...");
    status_fields(pr_state)
        .filter(|line| {
            line.strip_prefix(&prefix)
                .and_then(|suffix| suffix.split_whitespace().next())
                .is_some_and(|upstream| !upstream.starts_with('['))
        })
        .collect()
}

fn status_lines(value: &Value) -> Option<Vec<String>> {
    if let Some(text) = value.as_str() {
        let lines: Vec<_> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        return (!lines.is_empty()).then_some(lines);
    }
    value.as_array().map(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    })
}

fn is_dirty_status_line(line: &str) -> bool {
    let line = line.trim();
    !line.is_empty()
        && !line.starts_with("##")
        && !["clean", "working tree clean", "nothing to commit"]
            .iter()
            .any(|clean| line.eq_ignore_ascii_case(clean))
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}
