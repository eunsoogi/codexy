use serde_json::Value;

/// Extracts SHA-like values only from sentences that discuss pushed or synchronized heads.
pub(super) fn claimed_pushed_heads(text: &str) -> Vec<String> {
    let mut heads = text
        .split(['\n', ';'])
        .flat_map(|line| line.split(". "))
        .filter(|sentence| {
            let lower = sentence.to_ascii_lowercase();
            lower.contains("pushed")
                || lower.contains("synced")
                || lower.contains("local head")
                || lower.contains("remote/pr head match")
        })
        .flat_map(head_refs_after_markers)
        .collect::<Vec<_>>();
    heads.extend(table_heads(text));
    heads
}

/// A readable handoff may put the pushed state and its head on separate rows.
fn table_heads(text: &str) -> Vec<String> {
    super::child_handoff_readiness_table::rows(text)
        .into_iter()
        .filter(|rows| {
            rows.iter().any(|(key, value)| {
                let lower = value.to_ascii_lowercase();
                lower.contains("pushed")
                    || lower.contains("synced")
                    || key.eq_ignore_ascii_case("pushed")
            })
        })
        .flat_map(|rows| {
            rows.into_iter().filter_map(|(key, value)| {
                if !["head", "head sha", "local head"]
                    .iter()
                    .any(|name| key.eq_ignore_ascii_case(name))
                {
                    return None;
                }
                let value = value.trim().trim_matches('`');
                ((7..=40).contains(&value.len()) && value.chars().all(|ch| ch.is_ascii_hexdigit()))
                    .then(|| value.to_ascii_lowercase())
            })
        })
        .collect()
}

/// Requires current local and remote-tracking heads to match the PR head for readiness claims.
pub(super) fn captured_head_mismatch(pr_state: &Value) -> Option<String> {
    let Some(pr_head) = non_empty_string_field(pr_state, "headRefOid") else {
        return Some(
            "child handoff claims PR readiness but PR headRefOid evidence is missing".into(),
        );
    };
    let captured = [
        ("localHeadOid", "current local HEAD"),
        ("localHead", "current local HEAD"),
        ("gitHeadOid", "current local HEAD"),
        ("remoteHeadOid", "remote-tracking HEAD"),
        ("remoteHead", "remote-tracking HEAD"),
    ];
    if !captured.iter().any(|(field, label)| {
        label == &"current local HEAD" && non_empty_string_field(pr_state, field).is_some()
    }) {
        return Some(
            "child handoff claims PR readiness but current local HEAD evidence is missing".into(),
        );
    }
    if !captured.iter().any(|(field, label)| {
        label == &"remote-tracking HEAD" && non_empty_string_field(pr_state, field).is_some()
    }) {
        return Some(
            "child handoff claims PR readiness but remote-tracking HEAD evidence is missing".into(),
        );
    }
    captured
        .into_iter()
        .filter_map(|(field, label)| {
            non_empty_string_field(pr_state, field).map(|head| (label, head))
        })
        .find_map(|(label, head)| {
            (!heads_match(pr_head, head)).then(|| {
                format!(
                    "child handoff claims PR readiness but {label} is {head}, not PR headRefOid {pr_head}"
                )
            })
        })
}

fn heads_match(pr_head: &str, captured_head: &str) -> bool {
    let pr_head = pr_head.to_ascii_lowercase();
    let captured_head = captured_head.to_ascii_lowercase();
    if pr_head.len() < 7 || captured_head.len() < 7 {
        return false;
    }
    pr_head.starts_with(&captured_head) || captured_head.starts_with(&pr_head)
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn non_empty_string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    string_field(value, key)
        .map(str::trim)
        .filter(|head| !head.is_empty())
}

fn head_refs_after_markers(text: &str) -> Vec<String> {
    // Accept abbreviated hashes only after explicit head/commit/pushed markers, not arbitrary numbers.
    let mut refs = Vec::new();
    let (mut before_previous, mut previous) = (String::new(), String::new());
    for token in text.split_whitespace() {
        let token = token.trim_matches(|ch: char| ch == ',' || ch == '.' || ch == ')' || ch == '(');
        let (marker, value) = token
            .split_once('=')
            .map_or((None, token), |(marker, value)| (Some(marker), value));
        let candidate = value.trim_matches(|ch: char| !ch.is_ascii_hexdigit());
        let compact_candidate_looks_like_hash =
            candidate.len() == 40 || candidate.chars().any(|ch| ch.is_ascii_digit());
        let follows_named_marker = matches!(
            previous.as_str(),
            "at" | "head" | "head:" | "sha" | "sha:" | "commit" | "commit:"
        );
        let has_inline_marker = marker
            .map(|marker| {
                matches!(
                    marker
                        .trim_matches(|ch: char| !ch.is_ascii_alphabetic())
                        .to_ascii_lowercase()
                        .as_str(),
                    "head" | "sha" | "commit"
                )
            })
            .unwrap_or(false);
        let follows_compact_pushed_marker = compact_candidate_looks_like_hash
            && (matches!(previous.as_str(), "pushed" | "pushed:")
                || (matches!(previous.as_str(), "yes" | "yes:")
                    && matches!(before_previous.as_str(), "pushed" | "pushed:")));
        let follows_head_match_marker = matches!(previous.as_str(), "yes" | "yes:")
            && matches!(before_previous.as_str(), "match" | "match:");
        let is_pr_head = matches!(before_previous.as_str(), "pr" | "request")
            && matches!(previous.as_str(), "head" | "head:");
        // A claimed PR head is the comparison target, not evidence that the local or remote head was pushed.
        if (follows_named_marker
            || has_inline_marker
            || follows_compact_pushed_marker
            || follows_head_match_marker)
            && !is_pr_head
            && (7..=40).contains(&candidate.len())
            && candidate.chars().all(|ch| ch.is_ascii_hexdigit())
        {
            refs.push(candidate.to_ascii_lowercase());
        }
        before_previous = previous;
        previous = token
            .trim_matches(|ch: char| !ch.is_ascii_alphabetic() && ch != ':')
            .to_ascii_lowercase();
    }
    refs
}

pub(super) fn pushed_head_mismatch(handoff: &str, pr_state: &Value) -> Option<String> {
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
