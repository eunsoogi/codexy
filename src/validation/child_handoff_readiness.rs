use serde_json::Value;

pub(super) fn check(handoff: &str, pr_state: &Value) -> Vec<String> {
    let text = handoff.to_ascii_lowercase();
    if !claims_child_readiness(&text) {
        return Vec::new();
    }
    let mut errors = Vec::new();
    if claims_clean(&text) {
        match local_status(pr_state) {
            Some(lines) => {
                if lines.iter().any(|line| is_dirty_status_line(line)) {
                    errors.push(format!(
                        "child handoff claims clean worktree but current status is dirty: {}",
                        lines.join("; ")
                    ));
                }
            }
            None => errors.push(
                "child handoff claims clean worktree but current local git status evidence is missing"
                    .into(),
            ),
        }
    }
    if claims_synced_or_pushed(&text) {
        if let Some(status) = branch_divergence(pr_state) {
            errors.push(format!(
                "child handoff claims pushed/synced branch but current branch status is not pushed: {status}"
            ));
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
        "ready for parent handoff",
        "parent can open pr next: yes",
        "parent can merge",
    ]
    .iter()
    .any(|phrase| super::child_handoff_readiness_text::has_affirmed_phrase(text, phrase))
}

fn local_status(pr_state: &Value) -> Option<Vec<String>> {
    let statuses: Vec<_> = [
        "worktreeStatus",
        "localStatus",
        "gitStatus",
        "gitStatusShort",
        "statusShort",
    ]
    .into_iter()
    .filter_map(|field| pr_state.get(field))
    .filter_map(status_lines)
    .collect();
    (!statuses.is_empty()).then(|| statuses.into_iter().flatten().collect())
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

fn branch_divergence(pr_state: &Value) -> Option<String> {
    status_fields(pr_state).find(|line| {
        line.starts_with("##") && (line.contains("[ahead ") || line.contains("[behind "))
    })
}

fn pushed_head_mismatch(handoff: &str, pr_state: &Value) -> Option<String> {
    let Some(pr_head) = string_field(pr_state, "headRefOid").filter(|head| !head.is_empty()) else {
        return Some(
            "child handoff claims pushed/synced head but PR state is missing headRefOid".into(),
        );
    };
    let claimed = hex_refs(handoff);
    if claimed
        .iter()
        .any(|oid| pr_head.to_ascii_lowercase().starts_with(oid))
    {
        return None;
    }
    Some(format!(
        "child handoff claims pushed/synced head but PR headRefOid is {pr_head}, not {}",
        claimed
            .first()
            .map_or("any comparable handoff head", String::as_str)
    ))
}

fn unresolved_thread(pr_state: &Value) -> Option<String> {
    let nodes = pr_state.get("reviewThreads")?.get("nodes")?.as_array()?;
    nodes.iter().find_map(|thread| {
        (thread.get("isResolved").and_then(Value::as_bool) == Some(false))
            .then(|| thread_label(thread))
    })
}

fn thread_label(thread: &Value) -> String {
    let id = string_field(thread, "id").unwrap_or("unknown thread");
    let path = string_field(thread, "path").unwrap_or("unknown path");
    format!("{id} at {path}")
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn status_fields(pr_state: &Value) -> impl Iterator<Item = String> + '_ {
    [
        "worktreeStatus",
        "localStatus",
        "gitStatus",
        "gitStatusShort",
        "statusShort",
    ]
    .into_iter()
    .filter_map(|field| pr_state.get(field))
    .filter_map(status_lines)
    .flatten()
}

fn hex_refs(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_ascii_hexdigit())
        .filter(|part| (7..=40).contains(&part.len()))
        .map(str::to_ascii_lowercase)
        .collect()
}
