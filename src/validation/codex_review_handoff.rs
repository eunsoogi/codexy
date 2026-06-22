use serde_json::Value;

const READY_PHRASES: &str = "merge-ready|merge ready|ready to merge|ready for merge|ready for parent handoff|pr-ready|pr ready|pull-request-ready|pull request ready|codex review passed|codex review completed|codex review complete|codex review approved";
const OVERRIDE_PHRASES: &str = "maintainer override: yes|maintainer override: granted|maintainer accepted proceeding without codex review|maintainer accepted proceeding without full codex review|maintainer explicitly accepted proceeding without codex review|maintainer explicitly accepted proceeding without full codex review";

pub(super) fn check(handoff: &str, pr_state: &Value) -> Vec<String> {
    if claims_codex_review_ready(handoff)
        && has_latest_eyes_request_without_later_codex_output(pr_state)
        && !states_codex_review_override(handoff)
    {
        return vec![format!(
            "eyes-only Codex review request is not review completion: PR #{} needs actual Codex review output, an explicit completion signal, or a maintainer override before merge/readiness claims",
            pr_number(pr_state)
        )];
    }
    Vec::new()
}

fn claims_codex_review_ready(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    READY_PHRASES
        .split('|')
        .any(|phrase| has_affirmed_phrase(&text, phrase))
}

fn states_codex_review_override(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    OVERRIDE_PHRASES
        .split('|')
        .any(|phrase| has_affirmed_phrase(&text, phrase))
}

fn has_latest_eyes_request_without_later_codex_output(pr_state: &Value) -> bool {
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
    iter_json_objects(pr_state)
        .enumerate()
        .filter_map(|(order, item)| {
            let kind = if is_codex_review_request_with_eyes(item) {
                ReviewEventKind::EyesRequest
            } else if is_codex_review_output_item(item) {
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

fn is_codex_review_output_item(item: &Value) -> bool {
    if !is_codex_connector_item(item) {
        return false;
    }
    is_inline_review_comment_item(item)
        || text_field(item, "body")
            .or_else(|| text_field(item, "state"))
            .is_some_and(is_review_output_text)
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
fn has_affirmed_phrase(text: &str, phrase: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if is_boundary(text[..start].chars().next_back())
            && is_boundary(text[end..].chars().next())
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
        .rsplit_once(['.', '!', '?', ';', ':', '\n'])
        .map_or(prefix, |(_, clause)| clause);
    clause
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .rev()
        .take(4)
        .any(|word| matches!(word, "no" | "not" | "never" | "without"))
}

fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| !character.is_ascii_alphanumeric())
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

fn pr_number(pr_state: &Value) -> String {
    pr_state
        .get("number")
        .and_then(Value::as_u64)
        .map_or_else(|| "<unknown>".to_owned(), |number| number.to_string())
}
