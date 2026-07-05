use serde_json::Value;

const STATUS_FIELDS: [&str; 5] = [
    "worktreeStatus",
    "localStatus",
    "gitStatus",
    "gitStatusShort",
    "statusShort",
];

pub(super) fn check(handoff: &str, pr_state: &Value) -> Vec<String> {
    let text = handoff.to_ascii_lowercase();
    if !claims_child_readiness(&text) {
        return Vec::new();
    }
    let mut errors = Vec::new();
    if claims_clean(&text) || claims_pr_ready(&text) {
        let lines: Vec<_> = status_fields(pr_state).collect();
        if lines.is_empty() {
            errors.push(
                "child handoff claims clean/PR-ready worktree but current local git status evidence is missing".into(),
            );
        } else if lines.iter().any(|line| is_dirty_status_line(line)) {
            errors.push(format!(
                "child handoff claims clean/PR-ready worktree but current status is dirty: {}",
                lines.join("; ")
            ));
        }
    }
    if claims_synced_or_pushed(&text) {
        let statuses = pr_branch_statuses(pr_state);
        if statuses.is_empty() {
            errors.push(
                "child handoff claims pushed/synced branch but matching current branch status evidence is missing"
                    .into(),
            );
        } else {
            if let Some(status) = statuses.iter().find(|status| {
                status.contains("[ahead ")
                    || status.contains("[behind ")
                    || status.contains("[gone]")
            }) {
                errors.push(format!(
                    "child handoff claims pushed/synced branch but current branch status is not pushed: {status}"
                ));
            }
        }
        if let Some(error) = pushed_head_mismatch(handoff, pr_state) {
            errors.push(error);
        }
    }
    if claims_pr_ready(&text) {
        if let Some(state) = string_field(pr_state, "mergeStateStatus") {
            if !state.eq_ignore_ascii_case("CLEAN") {
                errors.push(format!(
                    "child handoff claims PR readiness but mergeStateStatus is {state}"
                ));
            }
        }
        let Some(threads) = pr_state.get("reviewThreads") else {
            errors.push(
                "child handoff claims PR readiness but reviewThreads.nodes PR state evidence is missing"
                    .into(),
            );
            return errors;
        };
        if threads.get("nodes").and_then(Value::as_array).is_none() {
            errors.push(
                "child handoff claims PR readiness but reviewThreads.nodes PR state evidence is missing"
                    .into(),
            );
            return errors;
        }
        if let Some(error) = super::review_thread_evidence::check(threads) {
            errors.push(error);
        } else if let Some(thread) = unresolved_thread(pr_state) {
            errors.push(format!(
                "child handoff claims PR readiness but unresolved review thread remains: {thread}"
            ));
        }
    }
    errors
}

fn claims_child_readiness(text: &str) -> bool {
    [
        "child handoff",
        "parent handoff",
        "pr ready for parent handoff",
        "parent can open pr next: yes",
        "parent can merge",
        "remote/pr head match: yes",
        "pushed: yes",
        "branch clean",
        "clean, synced",
        "synced, and pushed",
    ]
    .iter()
    .any(|phrase| super::child_handoff_readiness_text::has_affirmed_phrase(text, phrase))
}

fn claims_clean(text: &str) -> bool {
    [
        "branch clean",
        "worktree clean",
        "dirty state: clean",
        " clean,",
    ]
    .iter()
    .any(|phrase| super::child_handoff_readiness_text::has_affirmed_phrase(text, phrase))
}

fn claims_synced_or_pushed(text: &str) -> bool {
    ["synced", "pushed", "remote/pr head match: yes"]
        .iter()
        .any(|phrase| super::child_handoff_readiness_text::has_affirmed_phrase(text, phrase))
}

fn claims_pr_ready(text: &str) -> bool {
    [
        "pr ready",
        "pr-ready",
        "ready for parent handoff",
        "parent can open pr next: yes",
        "parent can merge",
    ]
    .iter()
    .any(|phrase| super::child_handoff_readiness_text::has_affirmed_phrase(text, phrase))
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

fn pr_branch_statuses(pr_state: &Value) -> Vec<String> {
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

fn pushed_head_mismatch(handoff: &str, pr_state: &Value) -> Option<String> {
    let Some(pr_head) = string_field(pr_state, "headRefOid").filter(|head| !head.is_empty()) else {
        return Some(
            "child handoff claims pushed/synced head but PR state is missing headRefOid".into(),
        );
    };
    let claimed = super::child_handoff_readiness_heads::claimed_pushed_heads(handoff);
    if claimed.is_empty() {
        return Some(format!(
            "child handoff claims pushed/synced head but PR headRefOid is {pr_head}, not any comparable handoff head"
        ));
    }
    if claimed
        .iter()
        .all(|oid| pr_head.to_ascii_lowercase().starts_with(oid))
    {
        return None;
    }
    let mismatched = claimed
        .iter()
        .find(|oid| !pr_head.to_ascii_lowercase().starts_with(*oid));
    Some(format!(
        "child handoff claims pushed/synced head but PR headRefOid is {pr_head}, not {}",
        mismatched.map_or("any comparable handoff head", String::as_str)
    ))
}

fn unresolved_thread(pr_state: &Value) -> Option<String> {
    let nodes = pr_state.get("reviewThreads")?.get("nodes")?.as_array()?;
    nodes.iter().find_map(|thread| {
        (thread.get("isResolved").and_then(Value::as_bool) == Some(false)).then(|| {
            format!(
                "{} at {}",
                string_field(thread, "id").unwrap_or("unknown thread"),
                string_field(thread, "path").unwrap_or("unknown path")
            )
        })
    })
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn status_fields(pr_state: &Value) -> impl Iterator<Item = String> + '_ {
    STATUS_FIELDS
        .into_iter()
        .filter_map(|field| pr_state.get(field))
        .filter_map(status_lines)
        .flatten()
}
