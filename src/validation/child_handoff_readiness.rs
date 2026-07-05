use serde_json::Value;

pub(super) fn check(handoff: &str, pr_state: &Value) -> Vec<String> {
    let text = handoff.to_ascii_lowercase();
    if !claims_child_readiness(&text) {
        return Vec::new();
    }
    let mut errors = Vec::new();
    if claims_clean(&text) || claims_pr_ready(&text) {
        match local_status(pr_state) {
            Some(lines) => {
                if lines.iter().any(|line| is_dirty_status_line(line)) {
                    errors.push(format!(
                        "child handoff claims clean/PR-ready worktree but current status is dirty: {}",
                        lines.join("; ")
                    ));
                }
            }
            None => errors.push(
                "child handoff claims clean/PR-ready worktree but current local git status evidence is missing"
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
        "pr-ready",
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
    let claimed = claimed_pushed_heads(handoff);
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

fn claimed_pushed_heads(text: &str) -> Vec<String> {
    text.split(|ch| matches!(ch, '\n' | ';'))
        .filter(|sentence| {
            sentence.to_ascii_lowercase().contains("pushed")
                || sentence.to_ascii_lowercase().contains("synced")
                || sentence.to_ascii_lowercase().contains("local head")
        })
        .flat_map(head_refs_after_markers)
        .collect()
}

fn head_refs_after_markers(text: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut previous = String::new();
    for token in text.split_whitespace() {
        let candidate = token.trim_matches(|ch: char| !ch.is_ascii_hexdigit());
        if matches!(
            previous.as_str(),
            "at" | "head" | "head:" | "sha" | "sha:" | "commit" | "commit:"
        ) && (7..=40).contains(&candidate.len())
            && candidate.chars().all(|ch| ch.is_ascii_hexdigit())
        {
            refs.push(candidate.to_ascii_lowercase());
        }
        previous = token
            .trim_matches(|ch: char| !ch.is_ascii_alphabetic() && ch != ':')
            .to_ascii_lowercase();
    }
    refs
}
