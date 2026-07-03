use serde_json::Value;

pub(super) fn check(handoff: &str, pr_state: &Value) -> Vec<String> {
    let text = handoff.to_ascii_lowercase();
    if !claims_child_readiness(&text) {
        return Vec::new();
    }
    let mut errors = Vec::new();
    if claims_clean(&text) {
        if let Some(status) = dirty_status(pr_state) {
            errors.push(format!(
                "child handoff claims clean worktree but current status is dirty: {status}"
            ));
        }
    }
    if claims_synced_or_pushed(&text) {
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
    .any(|phrase| has_affirmed_phrase(text, phrase))
}

fn claims_clean(text: &str) -> bool {
    [
        "branch clean",
        "worktree clean",
        "dirty state: clean",
        " clean,",
    ]
    .iter()
    .any(|phrase| has_affirmed_phrase(text, phrase))
}

fn claims_synced_or_pushed(text: &str) -> bool {
    ["synced", "pushed", "remote/pr head match: yes"]
        .iter()
        .any(|phrase| has_affirmed_phrase(text, phrase))
}

fn claims_pr_ready(text: &str) -> bool {
    [
        "pr ready",
        "ready for parent handoff",
        "parent can open pr next: yes",
        "parent can merge",
    ]
    .iter()
    .any(|phrase| has_affirmed_phrase(text, phrase))
}

fn dirty_status(pr_state: &Value) -> Option<String> {
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
    .find(|lines| lines.iter().any(|line| is_dirty_status_line(line)))
    .map(|lines| lines.join("; "))
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

fn pushed_head_mismatch(handoff: &str, pr_state: &Value) -> Option<String> {
    let pr_head = string_field(pr_state, "headRefOid")?;
    let claimed = hex_oids(handoff);
    if claimed.is_empty() || claimed.iter().any(|oid| oid == pr_head) {
        return None;
    }
    Some(format!(
        "child handoff claims pushed/synced head but PR headRefOid is {pr_head}, not {}",
        claimed.join(", ")
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

fn hex_oids(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_ascii_hexdigit())
        .filter(|part| part.len() == 40)
        .map(ToOwned::to_owned)
        .collect()
}

fn has_affirmed_phrase(text: &str, phrase: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if phrase_has_boundaries(text, start, end)
            && !is_locally_negated(&text[..start])
            && !super::codex_review_handoff::has_negative_label_value(&text[end..])
        {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

fn phrase_has_boundaries(text: &str, start: usize, end: usize) -> bool {
    is_boundary(text[..start].chars().next_back()) && is_boundary(text[end..].chars().next())
}

fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| {
        !character.is_ascii_alphanumeric() && character != '-' && character != '_'
    })
}

fn is_locally_negated(prefix: &str) -> bool {
    let clause_start = last_clause_boundary(prefix).map_or(0, |index| index);
    prefix[clause_start..]
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '\'')
        .filter(|word| !word.is_empty())
        .rev()
        .take(4)
        .any(|word| {
            matches!(
                word,
                "no" | "not"
                    | "never"
                    | "without"
                    | "isn't"
                    | "wasn't"
                    | "hasn't"
                    | "haven't"
                    | "aren't"
                    | "don't"
                    | "doesn't"
                    | "didn't"
                    | "won't"
                    | "can't"
                    | "cannot"
            )
        })
}

fn last_clause_boundary(text: &str) -> Option<usize> {
    text.char_indices()
        .filter(|(_, character)| matches!(character, '.' | '!' | '?' | ';' | ':' | ',' | '\n'))
        .map(|(index, character)| index + character.len_utf8())
        .last()
}
