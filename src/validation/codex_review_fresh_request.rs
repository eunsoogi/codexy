use serde_json::Value;

use super::codex_review_handoff_events as events;

pub(super) fn claims(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    text.lines().any(|line| {
        line.split([';', '.', '!', ','])
            .flat_map(request_subclauses)
            .any(|clause| {
                let clause = clause.trim();
                !has_negated_review_request(clause)
                    && !super::codex_review_fresh_request_text::has_negative_request_status(clause)
                    && !super::codex_review_fresh_request_text::is_connector_footer(clause)
                    && !is_review_request_status_clause(clause)
                    && !is_split_waiting_status(line, clause)
                    && is_review_request_clause(clause)
            })
    })
}

pub(super) fn is_review_request_clause(clause: &str) -> bool {
    let clause = clause.trim_start();
    if clause.contains("wait")
        && (clause.contains("review output") || clause.contains("post output"))
        && !has_review_request_action(clause)
    {
        return false;
    }
    let names_codex_review = clause.contains("codex review") || clause.contains("@codex review");
    names_codex_review
        && ((contains_word(clause, "request")
            || contains_word(clause, "requesting")
            || is_past_tense_codex_review_request_clause(clause))
            && !is_pull_request_noun_clause(clause)
            || super::codex_review_fresh_request_action::has_codex_review_post_action(clause)
            || starts_at_codex_review(clause.trim())
            || clause
                .strip_prefix("next action:")
                .is_some_and(starts_at_codex_review)
            || clause
                .strip_prefix("the ")
                .unwrap_or(clause)
                .strip_prefix("next action is to ")
                .is_some_and(starts_at_codex_review)
            || clause.starts_with("review request:"))
        || has_codex_review_request_variant(clause)
}

fn is_past_tense_codex_review_request_clause(clause: &str) -> bool {
    const PHRASES: &str = "requested @codex review|requested codex review|@codex review requested|codex review requested";
    PHRASES.split('|').any(|phrase| {
        clause.find(phrase).is_some_and(|index| {
            let after = clause[index + phrase.len()..].trim_start();
            !after.starts_with("change")
        })
    })
}

fn is_split_waiting_status(line: &str, clause: &str) -> bool {
    const STATUSES: &str = "waiting for review output|waiting for output|has eyes|eyes only";
    is_past_tense_codex_review_request_clause(clause)
        && line.find(clause).is_some_and(|index| {
            let rest = &line[index + clause.len()..];
            STATUSES.split('|').any(|status| rest.contains(status))
        })
}

#[rustfmt::skip]
fn starts_at_codex_review(text: &str) -> bool {
    text.trim_start().trim_start_matches(['`', '"', '\'']).starts_with("@codex review")
}

fn is_pull_request_noun_clause(clause: &str) -> bool {
    let clause = clause.trim_start();
    (clause.contains("pull request") || clause.contains("pr request"))
        && !has_codex_review_request_action(clause)
}

fn has_codex_review_request_action(clause: &str) -> bool {
    let mut rest = clause;
    let mut offset = 0;
    while let Some(index) = rest.find("request") {
        let start = offset + index;
        let end = start + "request".len();
        if is_word_match(clause, start, end)
            && !is_pull_request_noun_at(clause, start)
            && (clause[end..].contains("codex review") || clause[end..].contains("@codex review"))
        {
            return true;
        }
        offset = end;
        rest = &clause[offset..];
    }
    false
}

fn has_review_request_action(clause: &str) -> bool {
    has_codex_review_request_action(clause)
        || has_codex_review_request_variant(clause)
        || starts_at_codex_review(clause)
        || super::codex_review_fresh_request_action::has_codex_review_post_action(clause)
}

fn has_codex_review_request_variant(clause: &str) -> bool {
    clause.contains("request review from @codex")
        || clause.contains("request a review from @codex")
        || clause.contains("request @codex to review")
}

fn is_pull_request_noun_at(text: &str, request_start: usize) -> bool {
    let before = text[..request_start].trim_end();
    before.ends_with("pull") || before.ends_with("pr")
}

fn is_review_request_status_clause(clause: &str) -> bool {
    const PAST_TENSE_STATUSES: &str = "has eyes|eyes only|waiting for review output|requested for the current head|requested for current head";
    if is_past_tense_codex_review_request_clause(clause)
        && PAST_TENSE_STATUSES
            .split('|')
            .any(|status| clause.contains(status))
    {
        return true;
    }
    const REQUEST_STATUSES: &str = "request is pending|request pending|request has eyes|request has eyes only|request already has eyes|request is waiting|request exists|request already exists|request: pending|request: has eyes|request: has eyes only";
    (clause.contains("@codex review request") || clause.contains("codex review request"))
        && REQUEST_STATUSES
            .split('|')
            .any(|status| clause.contains(status))
}

pub(super) fn request_subclauses(clause: &str) -> impl Iterator<Item = &str> {
    let and = if is_past_tense_codex_review_request_clause(clause)
        && (clause.contains(" and waiting for review output") || clause.contains(" and has eyes"))
    {
        "\0"
    } else {
        " and "
    };
    clause
        .split(and)
        .flat_map(|part| part.split(" so "))
        .flat_map(|part| part.split(" then "))
        .flat_map(|part| part.split(" next action is to "))
}

fn contains_word(text: &str, word: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(word) {
        let start = offset + index;
        let end = start + word.len();
        if is_word_match(text, start, end) {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

#[rustfmt::skip]
pub(super) fn is_word_match(text: &str, start: usize, end: usize) -> bool {
    text[..start].chars().next_back().is_none_or(|ch| !ch.is_ascii_alphanumeric()) && text[end..].chars().next().is_none_or(|ch| !ch.is_ascii_alphanumeric())
}

#[rustfmt::skip]
pub(super) fn check(handoff: &str, pr_state: &Value) -> Option<String> {
    if !claims(handoff) { return None; }
    if let Some(error) = review_thread_evidence_error(pr_state) { return Some(format!("{error} before fresh Codex review requests: PR #{}", pr_number(pr_state))); }
    if missing_text(pr_state, "headRefOid") { return Some(format!("incomplete headRefOid PR state evidence before fresh Codex review requests: PR #{}", pr_number(pr_state))); }
    if missing_text(pr_state, "headRefCommittedDate") { return Some(format!("incomplete headRefCommittedDate PR state evidence before fresh Codex review requests: PR #{}", pr_number(pr_state))); }
    if !has_pr_comments_and_reviews_evidence(pr_state) { return Some(format!("fresh Codex review request evidence missing: include freshly captured PR comments and reviews before requesting @codex review on PR #{}", pr_number(pr_state))); }
    if events::has_latest_eyes_request_without_later_codex_output(pr_state) || events::has_codex_review_output(pr_state) { return Some(format!("current-head Codex review activity blocks fresh Codex review requests: PR #{} already has current-head request/output evidence", pr_number(pr_state))); }
    if has_blocking_unresolved_thread(handoff, pr_state) { return Some(format!("unresolved review thread blocks fresh Codex review requests: PR #{} must resolve or document accepted no-change rationale before requesting another @codex review", pr_number(pr_state))); }
    None
}
#[rustfmt::skip]
fn missing_text(pr_state: &Value, field: &str) -> bool {
    pr_state.get(field).and_then(Value::as_str).is_none_or(|text| text.trim().is_empty())
}
#[rustfmt::skip]
fn has_pr_comments_and_reviews_evidence(pr_state: &Value) -> bool {
    pr_state.get("comments").is_some_and(Value::is_array) && (pr_state.get("reviews").is_some_and(Value::is_array) || pr_state.get("latestReviews").is_some_and(Value::is_array))
}

pub(super) fn review_thread_evidence_error(pr_state: &Value) -> Option<String> {
    let Some(threads) = pr_state.get("reviewThreads") else {
        return Some(
            "incomplete reviewThreads.nodes PR state evidence: missing reviewThreads".into(),
        );
    };
    if threads.get("nodes").and_then(Value::as_array).is_none() {
        return Some("incomplete reviewThreads.nodes PR state evidence: missing nodes".into());
    }
    super::review_thread_evidence::check(threads)
}

pub(super) fn has_blocking_unresolved_thread(handoff: &str, pr_state: &Value) -> bool {
    pr_state
        .get("reviewThreads")
        .and_then(|threads| threads.get("nodes"))
        .and_then(Value::as_array)
        .is_some_and(|nodes| {
            nodes.iter().any(|thread| {
                thread.get("isResolved").and_then(Value::as_bool) == Some(false)
                    && !super::review_thread_resolution::documents_accepted_no_change_rationale(
                        handoff, thread,
                    )
            })
        })
}

fn pr_number(pr_state: &Value) -> String {
    pr_state
        .get("number")
        .and_then(Value::as_u64)
        .map_or_else(|| "<unknown>".to_owned(), |number| number.to_string())
}

pub(super) fn has_negated_review_request(clause: &str) -> bool {
    if clause
        .rsplit_once(':')
        .is_some_and(|(_, action)| is_post_colon_request_action(action.trim()))
    {
        return false;
    }
    const PHRASES: &str = "do not request|don't request|do not post|don't post|do not comment|don't comment|do not send|don't send|before requesting|not requested @codex review|not requested codex review|@codex review not requested|codex review not requested|no @codex review requested|no codex review requested|no @codex review request|no codex review request|no current-head @codex review request|no current-head codex review request|no current-head request|no current head @codex review request|no current head codex review request|no current head request|no request|without @codex review request|without codex review request|without current-head @codex review request|without current-head codex review request|not request|without current-head request|without current head @codex review request|without current head codex review request|without current head request|without request|will not request|won't request|must not request|mustn't request|will not post|won't post|must not post|mustn't post|will not comment|won't comment|must not comment|mustn't comment|will not send|won't send|must not send|mustn't send|not ready to request|not yet ready to request|not currently ready to request|isn't ready to request|isn't yet ready to request|isn't currently ready to request|aren't ready to request|aren't yet ready to request|aren't currently ready to request";
    PHRASES.split('|').any(|phrase| clause.contains(phrase))
}

fn is_post_colon_request_action(action: &str) -> bool {
    let action = action.strip_prefix("please ").unwrap_or(action);
    (action.starts_with("request ")
        || action.starts_with("post ")
        || action.starts_with("comment ")
        || action.starts_with("send ")
        || starts_at_codex_review(action))
        && is_review_request_clause(action)
}
