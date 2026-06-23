use serde_json::Value;

pub(super) fn has_unresolved_codex_review_thread(pr_state: &Value) -> bool {
    iter_json_objects(pr_state).any(|item| {
        item.get("isResolved").and_then(Value::as_bool) == Some(false)
            && item.get("isOutdated").and_then(Value::as_bool) != Some(true)
            && iter_json_objects(item).any(is_codex_connector_item)
    })
}

pub(super) fn has_codex_review_output(pr_state: &Value) -> bool {
    review_events(pr_state)
        .iter()
        .any(|event| matches!(event.kind, ReviewEventKind::CodexOutput))
}

pub(super) fn has_latest_eyes_request_without_later_codex_output(pr_state: &Value) -> bool {
    let events = review_events(pr_state);
    let Some(latest_eyes_request) = events
        .iter()
        .enumerate()
        .filter(|(_, event)| matches!(event.kind, ReviewEventKind::EyesRequest))
        .max_by(|(_, left), (_, right)| compare_event_order(left, right))
        .map(|(index, _)| index)
    else {
        return false;
    };
    !events
        .iter()
        .enumerate()
        .filter(|(_, event)| matches!(event.kind, ReviewEventKind::CodexOutput))
        .filter(|(_, event)| is_after_event(event, &events[latest_eyes_request]))
        .any(|(index, _)| index != latest_eyes_request)
}

fn review_events(pr_state: &Value) -> Vec<ReviewEvent<'_>> {
    let head = text_field(pr_state, "headRefOid");
    iter_json_objects(pr_state)
        .enumerate()
        .filter_map(|(order, item)| {
            let kind = if is_codex_review_request_with_eyes(item) {
                ReviewEventKind::EyesRequest
            } else if is_codex_review_output_item(item, head) {
                ReviewEventKind::CodexOutput
            } else {
                return None;
            };
            Some(ReviewEvent {
                kind,
                timestamp: event_timestamp(item),
                order,
            })
        })
        .collect()
}

#[derive(Clone, Copy)]
struct ReviewEvent<'a> {
    kind: ReviewEventKind,
    timestamp: Option<&'a str>,
    order: usize,
}

#[derive(Clone, Copy)]
enum ReviewEventKind {
    EyesRequest,
    CodexOutput,
}

fn is_after_event(event: &ReviewEvent<'_>, baseline: &ReviewEvent<'_>) -> bool {
    matches!((event.timestamp, baseline.timestamp), (Some(event), Some(baseline)) if event > baseline)
}

fn compare_event_order(left: &ReviewEvent<'_>, right: &ReviewEvent<'_>) -> std::cmp::Ordering {
    match (left.timestamp, right.timestamp) {
        (Some(left), Some(right)) if left != right => left.cmp(right),
        _ => left.order.cmp(&right.order),
    }
}

fn event_timestamp(item: &Value) -> Option<&str> {
    ["createdAt", "submittedAt", "updatedAt"]
        .iter()
        .find_map(|field| text_field(item, field))
}

fn is_codex_review_request_with_eyes(item: &Value) -> bool {
    text_field(item, "body").is_some_and(|body| body.contains("@codex review"))
        && has_eyes_reaction(item)
}

fn is_codex_review_output_item(item: &Value, head: Option<&str>) -> bool {
    if !is_codex_connector_item(item) {
        return false;
    }
    (is_inline_review_comment_item(item)
        || text_field(item, "body")
            .filter(|text| !text.trim().is_empty())
            .or_else(|| text_field(item, "state"))
            .is_some_and(is_review_output_text))
        && codex_output_matches_head(item, head)
}

fn codex_output_matches_head(item: &Value, head: Option<&str>) -> bool {
    let Some(head) = head else { return true };
    let Some(oid) = item
        .get("commit")
        .and_then(|commit| text_field(commit, "oid"))
        .filter(|oid| !oid.is_empty())
        .or_else(|| text_field(item, "body").and_then(reviewed_commit))
    else {
        return false;
    };
    head.starts_with(oid) || oid.starts_with(head)
}

fn reviewed_commit(text: &str) -> Option<&str> {
    text.split("Reviewed commit").nth(1)?.split('`').nth(1)
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

fn is_review_output_text(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    let output = "didn't find any major issues|no major issues|no actionable issues|no suggestions|no issues|suggestion|review complete|review completed|completed review|finished review|looks good|actionable issue|approved|+1";
    !text.trim().eq("@codex review")
        && !text.contains("create an environment for this repo")
        && !is_review_progress_text(&text)
        && output.split('|').any(|phrase| text.contains(phrase))
}

fn is_review_progress_text(text: &str) -> bool {
    let future = "will post|will provide|will add|i'll post|i will post|when complete|once complete|after review completes|review is still running|review is in progress|review started";
    let result = "suggestion|finding|issue|comment";
    future.split('|').any(|phrase| text.contains(phrase))
        && result.split('|').any(|phrase| text.contains(phrase))
}

fn has_eyes_reaction(item: &Value) -> bool {
    item.get("reactionGroups")
        .and_then(Value::as_array)
        .is_some_and(|groups| {
            groups.iter().any(|group| {
                text_field(group, "content").is_some_and(|content| {
                    content.eq_ignore_ascii_case("EYES")
                        && group
                            .get("users")
                            .and_then(|users| users.get("totalCount"))
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                            > 0
                })
            })
        })
        || item
            .get("reactions")
            .is_some_and(|reactions| match reactions {
                Value::Object(map) => map
                    .get("eyes")
                    .and_then(Value::as_u64)
                    .is_some_and(|count| count > 0),
                Value::Array(items) => items.iter().any(|reaction| {
                    text_field(reaction, "content")
                        .or_else(|| text_field(reaction, "name"))
                        .is_some_and(|content| content.eq_ignore_ascii_case("eyes"))
                }),
                _ => false,
            })
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
