use serde_json::Value;

use super::codex_review_handoff_output::{is_review_output_text, iter_json_objects, oid_matches};

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

pub(super) fn has_pending_codex_review_request_or_current_head_output(pr_state: &Value) -> bool {
    let head = text_field(pr_state, "headRefOid");
    iter_json_objects(pr_state).any(|item| is_codex_review_output_item(item, head))
        || latest_request_without_later_output(pr_state, false)
}

pub(super) fn has_latest_eyes_request_without_later_codex_output(pr_state: &Value) -> bool {
    latest_request_without_later_output(pr_state, true)
}

fn latest_request_without_later_output(pr_state: &Value, current_head_output_only: bool) -> bool {
    let head = text_field(pr_state, "headRefOid");
    let head_timestamp = text_field(pr_state, "headRefCommittedDate");
    let events = review_events(pr_state, current_head_output_only);
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
    !events.iter().enumerate().any(|(index, event)| {
        index != latest_eyes_request
            && matches!(event.kind, ReviewEventKind::CodexOutput)
            && is_after_event(event, request)
            && output_can_fulfill_latest_request(event, request, &events, head, head_timestamp)
    })
}

fn review_events(pr_state: &Value, current_head_output_only: bool) -> Vec<ReviewEvent<'_>> {
    let head = text_field(pr_state, "headRefOid");
    iter_json_objects(pr_state)
        .enumerate()
        .filter_map(|(order, item)| {
            let kind = if is_codex_review_request(item) {
                ReviewEventKind::CodexRequest
            } else if current_head_output_only && is_codex_review_output_item(item, head)
                || !current_head_output_only
                    && is_codex_connector_item(item)
                    && is_review_output_signal(item)
            {
                ReviewEventKind::CodexOutput
            } else {
                return None;
            };
            Some(ReviewEvent {
                kind,
                timestamp: event_timestamp(item),
                commit: output_commit(item),
                order,
            })
        })
        .collect()
}

#[derive(Clone, Copy)]
struct ReviewEvent<'a> {
    kind: ReviewEventKind,
    timestamp: Option<&'a str>,
    commit: Option<&'a str>,
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

fn output_can_fulfill_latest_request(
    event: &ReviewEvent<'_>,
    request: &ReviewEvent<'_>,
    events: &[ReviewEvent<'_>],
    head: Option<&str>,
    head_timestamp: Option<&str>,
) -> bool {
    let request_before_current_head = matches!((request.timestamp, head_timestamp), (Some(request), Some(head)) if request < head);
    let Some(commit) = event.commit else {
        return request_before_current_head;
    };
    let current_head_output = head.is_some_and(|head| oid_matches(head, commit));
    let stale_for_current_head =
        !current_head_output && head.is_some() && !request_before_current_head;
    !stale_for_current_head
        && (current_head_output
            || !events.iter().any(|prior| {
                matches!(prior.kind, ReviewEventKind::CodexOutput)
                    && prior.commit == Some(commit)
                    && is_after_event(request, prior)
            }))
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
        && is_pr_comment_item(item)
        && text_field(item, "body").is_some_and(|body| body.contains("@codex review"))
}

fn is_pr_comment_item(item: &Value) -> bool {
    ["url", "html_url"]
        .iter()
        .filter_map(|field| text_field(item, field))
        .any(|url| url.contains("#issuecomment-"))
}

fn is_codex_review_output_item(item: &Value, head: Option<&str>) -> bool {
    is_codex_connector_item(item)
        && is_review_output_signal(item)
        && codex_output_matches_head(item, head)
}

fn is_review_output_signal(item: &Value) -> bool {
    is_inline_review_comment_item(item)
        || text_field(item, "state").is_some_and(is_review_output_text)
        || text_field(item, "body")
            .filter(|text| !text.trim().is_empty())
            .is_some_and(is_review_output_text)
}

fn codex_output_matches_head(item: &Value, head: Option<&str>) -> bool {
    let Some(head) = head.filter(|head| !head.trim().is_empty()) else {
        return false;
    };
    output_commit(item).is_some_and(|oid| oid_matches(head, oid))
}

fn output_commit(item: &Value) -> Option<&str> {
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

fn text_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}
