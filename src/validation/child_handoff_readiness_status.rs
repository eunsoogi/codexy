use serde_json::Value;

const STATUS_FIELDS: [&str; 5] = [
    "worktreeStatus",
    "localStatus",
    "gitStatus",
    "gitStatusShort",
    "statusShort",
];

pub(super) fn dirty_status(pr_state: &Value) -> Option<String> {
    STATUS_FIELDS
        .into_iter()
        .filter_map(|field| pr_state.get(field))
        .filter_map(status_lines)
        .find(|lines| lines.iter().any(|line| is_dirty_status_line(line)))
        .map(|lines| lines.join("; "))
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

pub(super) fn branch_diverged(status: &str) -> bool {
    ["[ahead ", "[behind ", "[gone]"]
        .iter()
        .any(|marker| status.contains(marker))
}

fn status_lines(value: &Value) -> Option<Vec<String>> {
    if let Some(text) = value.as_str() {
        return Some(text.lines().map(str::trim).map(ToOwned::to_owned).collect());
    }
    value.as_array().map(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
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

fn status_fields(pr_state: &Value) -> impl Iterator<Item = String> + '_ {
    STATUS_FIELDS
        .into_iter()
        .filter_map(|field| pr_state.get(field))
        .filter_map(status_lines)
        .flatten()
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}
