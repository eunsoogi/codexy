use serde_json::{Value, json};

pub(crate) fn legacy_control(profile: &str, issue_number: u64, head: &str) -> Value {
    let (name, model, effort) = legacy_reviewer(profile);
    json!({
        "schema": "codexy.review-control-state.v1",
        "issue_number": issue_number,
        "profile": profile,
        "reviewer": reviewer(name, model, effort),
        "reviewed_head": head,
        "terminal_result": "PASS",
        "unresolved_findings": [],
        "full_review_count": 1,
        "delta_review_count": 0,
        "terminal_review_count": 1,
        "terminal_review_limit": 3,
        "terminal_review_history": [event(
            &format!("{profile}-legacy-full-1"),
            "full",
            name,
            model,
            effort,
            head,
            "PASS",
        )]
    })
}

pub(crate) fn migrated_control(
    profile: &str,
    issue_number: u64,
    previous_head: &str,
    head: &str,
) -> Value {
    let (name, old_model, old_effort) = legacy_reviewer(profile);
    let (_, current_model, current_effort) = current_reviewer(profile);
    json!({
        "schema": "codexy.review-control-state.v1",
        "issue_number": issue_number,
        "profile": profile,
        "reviewer": reviewer(name, current_model, current_effort),
        "reviewed_head": head,
        "terminal_result": "PASS",
        "unresolved_findings": [],
        "full_review_count": 1,
        "delta_review_count": 1,
        "terminal_review_count": 2,
        "terminal_review_limit": 3,
        "terminal_review_history": [
            event(
                &format!("{profile}-legacy-full-1"),
                "full",
                name,
                old_model,
                old_effort,
                previous_head,
                "PASS",
            ),
            event(
                &format!("{profile}-current-delta-1"),
                "delta",
                name,
                current_model,
                current_effort,
                head,
                "PASS",
            )
        ]
    })
}

pub(crate) fn continued_control(
    profile: &str,
    issue_number: u64,
    full_head: &str,
    delta_head: &str,
    current_head: &str,
) -> Value {
    let (name, old_model, old_effort) = legacy_reviewer(profile);
    let (_, current_model, current_effort) = current_reviewer(profile);
    json!({
        "schema": "codexy.review-control-state.v1",
        "issue_number": issue_number,
        "profile": profile,
        "reviewer": reviewer(name, current_model, current_effort),
        "reviewed_head": current_head,
        "terminal_result": "PASS",
        "unresolved_findings": [],
        "full_review_count": 1,
        "delta_review_count": 1,
        "terminal_review_count": 3,
        "terminal_review_limit": 3,
        "terminal_review_history": [
            event(
                &format!("{profile}-legacy-full-1"),
                "full",
                name,
                old_model,
                old_effort,
                full_head,
                "PASS",
            ),
            event(
                &format!("{profile}-current-delta-1"),
                "delta",
                name,
                current_model,
                current_effort,
                delta_head,
                "PASS",
            ),
            event(
                &format!("{profile}-current-required-head-1"),
                "required_current_head",
                name,
                current_model,
                current_effort,
                current_head,
                "PASS",
            )
        ],
        "post_cap_re_review": {
            "reason": "mandatory_base_integration",
            "prior_reviewed_head": delta_head,
            "qualifying_change": {
                "from_head": delta_head,
                "to_head": current_head,
                "evidence_commit": "synthetic-integration-evidence",
                "finding_ids": []
            }
        }
    })
}

fn legacy_reviewer(profile: &str) -> (&'static str, &'static str, &'static str) {
    match profile {
        "strict" => ("codexy-sentinel", "gpt-5.6-sol", "xhigh"),
        "standard" => ("codexy-inspector", "gpt-5.6-terra", "max"),
        _ => unreachable!("known profile"),
    }
}

fn current_reviewer(profile: &str) -> (&'static str, &'static str, &'static str) {
    match profile {
        "strict" => ("codexy-sentinel", "gpt-6-astra", "xhigh"),
        "standard" => ("codexy-inspector", "gpt-5.6-sol", "medium"),
        _ => unreachable!("known profile"),
    }
}

fn reviewer(name: &str, model: &str, effort: &str) -> Value {
    json!({"name": name, "model": model, "reasoning_effort": effort})
}

fn event(
    id: &str,
    kind: &str,
    name: &str,
    model: &str,
    effort: &str,
    head: &str,
    result: &str,
) -> Value {
    json!({
        "id": id,
        "kind": kind,
        "reviewer": reviewer(name, model, effort),
        "reviewed_head": head,
        "terminal_result": result,
        "unresolved_findings": []
    })
}
