use serde_json::Value;

pub(super) fn has_actionable_codex_review_output(pr_state: &Value) -> bool {
    let head = text_field(pr_state, "headRefOid");
    iter_json_objects(pr_state).any(|item| {
        is_codex_connector_item(item)
            && !is_inline_review_comment_item(item)
            && codex_output_matches_head(item, head)
            && text_field(item, "body").is_some_and(is_actionable_review_output_text)
    })
}

fn codex_output_matches_head(item: &Value, head: Option<&str>) -> bool {
    let Some(head) = head.filter(|head| !head.trim().is_empty()) else {
        return false;
    };
    let Some(oid) = item
        .get("commit")
        .and_then(|commit| text_field(commit, "oid"))
        .filter(|oid| is_commit_oid(oid))
        .or_else(|| text_field(item, "body").and_then(reviewed_commit))
    else {
        return false;
    };
    head.starts_with(oid) || oid.starts_with(head)
}

fn reviewed_commit(text: &str) -> Option<&str> {
    text.split("Reviewed commit")
        .nth(1)?
        .split('`')
        .nth(1)
        .filter(|oid| is_commit_oid(oid))
}

fn is_commit_oid(oid: &str) -> bool {
    (7..=40).contains(&oid.len()) && oid.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_inline_review_comment_item(item: &Value) -> bool {
    ["url", "html_url"]
        .iter()
        .filter_map(|field| text_field(item, field))
        .any(|url| url.contains("#discussion_r"))
        || item.get("path").is_some()
            && ["line", "position", "originalLine", "original_line"]
                .iter()
                .any(|field| item.get(*field).is_some())
}

fn is_codex_connector_item(item: &Value) -> bool {
    ["author", "user"]
        .iter()
        .filter_map(|field| item.get(field))
        .any(is_codex_connector_identity)
        || item
            .get("performed_via_github_app")
            .is_some_and(is_codex_connector_identity)
}

fn is_codex_connector_identity(value: &Value) -> bool {
    text_field(value, "slug").is_some_and(|slug| slug == "chatgpt-codex-connector")
        || text_field(value, "login").is_some_and(|login| {
            login == "chatgpt-codex-connector" || login == "chatgpt-codex-connector[bot]"
        })
}

fn is_actionable_review_output_text(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    !is_review_progress_text(&text)
        && [
            "actionable issue",
            "actionable issues",
            "suggestion",
            "suggestions",
        ]
        .iter()
        .any(|phrase| has_unnegated_phrase(&text, phrase))
}

fn has_unnegated_phrase(text: &str, phrase: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if is_phrase_boundary(text[..start].chars().next_back())
            && is_phrase_boundary(text[end..].chars().next())
            && !is_locally_negated(&text[..start])
        {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

fn is_locally_negated(prefix: &str) -> bool {
    let clause = prefix
        .rsplit_once(['.', '!', '?', ';', ':', ',', '\n'])
        .map_or(prefix, |(_, clause)| clause);
    clause
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '\'')
        .filter(|word| !word.is_empty())
        .rev()
        .take(4)
        .any(|word| matches!(word, "no" | "not" | "without" | "didn't"))
}

fn is_phrase_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn is_review_progress_text(text: &str) -> bool {
    let future = "will post|will provide|will add|i'll post|i will post|when complete|once complete|after review completes|review is still running|review is in progress|review started";
    let result = "suggestion|finding|issue|comment";
    future.split('|').any(|phrase| text.contains(phrase))
        && result.split('|').any(|phrase| text.contains(phrase))
}

fn text_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn iter_json_objects(value: &Value) -> Box<dyn Iterator<Item = &Value> + '_> {
    match value {
        Value::Object(map) => Box::new(
            std::iter::once(value).chain(map.values().flat_map(|value| iter_json_objects(value))),
        ),
        Value::Array(items) => Box::new(items.iter().flat_map(|value| iter_json_objects(value))),
        _ => Box::new(std::iter::empty()),
    }
}
