use serde_json::Value;
pub(super) fn has_unresolved_codex_review_thread(pr_state: &Value) -> bool {
    iter_json_objects(pr_state).any(|item| {
        item.get("isResolved").and_then(Value::as_bool) == Some(false)
            && item.get("isOutdated").and_then(Value::as_bool) != Some(true)
            && (iter_json_objects(item).any(is_codex_connector_item)
                || unresolved_thread_lacks_comment_identity(item))
    })
}
pub(super) fn has_codex_review_output(pr_state: &Value) -> bool {
    review_events(pr_state, true)
        .iter()
        .any(|event| matches!(event.kind, ReviewEventKind::CodexOutput))
}
pub(super) fn has_codex_review_activity(pr_state: &Value) -> bool {
    iter_json_objects(pr_state).any(|item| {
        is_codex_review_request(item)
            || is_codex_connector_item(item) && is_review_output_signal(item)
    })
}
pub(super) fn has_latest_eyes_request_without_later_codex_output(pr_state: &Value) -> bool {
    let events = review_events(pr_state, false);
    let Some(latest_eyes_request) = events
        .iter()
        .enumerate()
        .filter(|(_, event)| matches!(event.kind, ReviewEventKind::CodexRequest))
        .max_by(|(_, left), (_, right)| compare_event_order(left, right))
        .map(|(index, _)| index)
    else {
        return false;
    };
    let request = &events[latest_eyes_request];
    !events
        .iter()
        .enumerate()
        .filter(|(_, event)| matches!(event.kind, ReviewEventKind::CodexOutput))
        .filter(|(_, event)| is_after_event(event, request))
        .filter(|(_, event)| event_clears_request(event, request))
        .any(|(index, _)| index != latest_eyes_request)
}
fn review_events(pr_state: &Value, require_head_match: bool) -> Vec<ReviewEvent<'_>> {
    let head = text_field(pr_state, "headRefOid");
    iter_json_objects(pr_state)
        .enumerate()
        .filter_map(|(order, item)| {
            let matches_head = codex_item_matches_head(item, head);
            let kind = if is_codex_review_request(item) {
                ReviewEventKind::CodexRequest
            } else if is_codex_connector_item(item)
                && is_review_output_signal(item)
                && (!require_head_match || matches_head)
            {
                ReviewEventKind::CodexOutput
            } else {
                return None;
            };
            Some(ReviewEvent {
                kind,
                matches_head,
                has_head_evidence: codex_item_reviewed_oid(item).is_some(),
                timestamp: event_timestamp(item),
                order,
            })
        })
        .collect()
}
#[derive(Clone, Copy)]
struct ReviewEvent<'a> {
    kind: ReviewEventKind,
    matches_head: bool,
    has_head_evidence: bool,
    timestamp: Option<&'a str>,
    order: usize,
}
#[derive(Clone, Copy)]
enum ReviewEventKind {
    CodexRequest,
    CodexOutput,
}
fn is_after_event(event: &ReviewEvent<'_>, baseline: &ReviewEvent<'_>) -> bool {
    matches!((event.timestamp, baseline.timestamp), (Some(event), Some(baseline)) if event > baseline)
}
fn event_clears_request(event: &ReviewEvent<'_>, request: &ReviewEvent<'_>) -> bool {
    !request.has_head_evidence && event.has_head_evidence
        || request.has_head_evidence && (!request.matches_head || event.matches_head)
}
fn compare_event_order(left: &ReviewEvent<'_>, right: &ReviewEvent<'_>) -> std::cmp::Ordering {
    match (left.timestamp, right.timestamp) {
        (Some(left), Some(right)) if left != right => left.cmp(right),
        _ => left.order.cmp(&right.order),
    }
}
fn event_timestamp(item: &Value) -> Option<&str> {
    "createdAt|submittedAt|updatedAt|created_at|submitted_at|updated_at"
        .split('|')
        .find_map(|field| text_field(item, field))
}
fn unresolved_thread_lacks_comment_identity(thread: &Value) -> bool {
    thread
        .get("comments")
        .and_then(|comments| comments.get("nodes"))
        .and_then(Value::as_array)
        .is_none_or(|comments| {
            comments
                .iter()
                .any(|comment| !has_comment_identity(comment))
        })
}
fn has_comment_identity(comment: &Value) -> bool {
    ["author", "user", "performed_via_github_app"]
        .iter()
        .filter_map(|field| comment.get(*field))
        .any(has_concrete_identity)
}
fn has_concrete_identity(value: &Value) -> bool {
    text_field(value, "login")
        .or_else(|| text_field(value, "slug"))
        .is_some_and(|identity| !identity.trim().is_empty())
}
fn is_codex_review_request(item: &Value) -> bool {
    !is_codex_connector_item(item)
        && has_eyes_reaction(item)
        && text_field(item, "body").is_some_and(has_actionable_codex_review_request_text)
}
fn has_eyes_reaction(item: &Value) -> bool {
    item.get("reactionGroups")
        .and_then(Value::as_array)
        .is_some_and(|groups| {
            groups.iter().any(|group| {
                text_field(group, "content") == Some("EYES")
                    && group
                        .get("users")
                        .and_then(|users| users.get("totalCount").and_then(Value::as_u64))
                        .is_some_and(|count| count > 0)
            })
        })
        || has_rest_eyes_reaction(item)
}
fn has_rest_eyes_reaction(item: &Value) -> bool {
    match item.get("reactions") {
        Some(Value::Array(reactions)) => reactions
            .iter()
            .any(|reaction| text_field(reaction, "content") == Some("eyes")),
        Some(Value::Object(reactions)) => reactions
            .get("eyes")
            .and_then(Value::as_u64)
            .is_some_and(|count| count > 0),
        _ => false,
    }
}
fn has_actionable_codex_review_request_text(body: &str) -> bool {
    body.to_ascii_lowercase()
        .lines()
        .flat_map(|line| line.split([';', '.', '!', ',']))
        .flat_map(super::codex_review_fresh_request::request_subclauses)
        .map(str::trim)
        .any(|clause| {
            super::codex_review_fresh_request::is_review_request_clause(clause)
                && !super::codex_review_fresh_request::has_negated_review_request(clause)
                && !super::codex_review_fresh_request_text::is_connector_footer(clause)
                && !super::codex_review_fresh_request_text::has_negative_request_status(clause)
        })
}
fn is_review_output_signal(item: &Value) -> bool {
    is_inline_review_comment_item(item)
        || text_field(item, "state").is_some_and(is_review_output_text)
        || text_field(item, "body")
            .filter(|text| !text.trim().is_empty())
            .is_some_and(is_review_output_text)
}
fn codex_item_matches_head(item: &Value, head: Option<&str>) -> bool {
    let Some(head) = head.filter(|head| !head.trim().is_empty()) else {
        return false;
    };
    let Some(oid) = codex_item_reviewed_oid(item) else {
        return false;
    };
    head.starts_with(oid) || oid.starts_with(head)
}
fn codex_item_reviewed_oid(item: &Value) -> Option<&str> {
    item.get("commit")
        .and_then(|commit| text_field(commit, "oid"))
        .filter(|oid| is_commit_oid(oid))
        .or_else(|| text_field(item, "body").and_then(reviewed_commit))
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
