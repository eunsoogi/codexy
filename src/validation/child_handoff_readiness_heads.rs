use serde_json::Value;

pub(super) fn claimed_pushed_heads(text: &str) -> Vec<String> {
    text.split(|ch| matches!(ch, '\n' | ';'))
        .flat_map(|line| line.split(". "))
        .filter(|sentence| {
            let lower = sentence.to_ascii_lowercase();
            lower.contains("pushed")
                || lower.contains("synced")
                || lower.contains("local head")
                || lower.contains("remote/pr head match")
        })
        .flat_map(head_refs_after_markers)
        .collect()
}

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
