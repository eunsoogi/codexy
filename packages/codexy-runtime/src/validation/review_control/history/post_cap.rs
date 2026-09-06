use std::collections::HashSet;

use serde_json::Value;

use super::{reject_unknown, required_text};
use crate::validation::review_control::external_finding;

pub(super) fn check(
    value: &Value,
    prior_head: Option<&Value>,
    reviewed_head: &str,
) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "review control state post_cap_re_review must be an object".to_owned())?;
    reject_unknown(
        object,
        &["reason", "prior_reviewed_head", "qualifying_change"],
        "post_cap_re_review",
    )?;
    let reason = required_text(object, "reason", "post_cap_re_review")?;
    if !matches!(
        reason,
        "mandatory_base_integration" | "in_scope_contract_root_repair" | external_finding::REASON
    ) {
        return Err("review control state post-cap reason is not eligible".into());
    }
    let prior = required_text(object, "prior_reviewed_head", "post_cap_re_review")?;
    if prior == reviewed_head {
        return Err(
            "review control state post-cap re-review must advance the reviewed head".into(),
        );
    }
    if prior_head.and_then(Value::as_str) != Some(prior) {
        return Err("review control state post-cap re-review does not bind the delta head".into());
    }
    let change = object
        .get("qualifying_change")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            "review control state post-cap re-review must record qualifying change evidence"
                .to_owned()
        })?;
    reject_unknown(
        change,
        &[
            "from_head",
            "to_head",
            "evidence_commit",
            "finding_ids",
            "external_finding",
        ],
        "post_cap_re_review.qualifying_change",
    )?;
    if required_text(change, "from_head", "qualifying_change")? != prior
        || required_text(change, "to_head", "qualifying_change")? != reviewed_head
    {
        return Err(
            "review control state qualifying change evidence does not bind the heads".into(),
        );
    }
    let evidence_commit = required_text(change, "evidence_commit", "qualifying_change")?;
    if evidence_commit.len() != 40 || !evidence_commit.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("review control state qualifying change evidence must name a commit".into());
    }
    let finding_ids = change.get("finding_ids");
    if let Some(value) = finding_ids {
        let ids = value.as_array().ok_or_else(|| {
            "review control state qualifying change finding_ids must be an array".to_owned()
        })?;
        let mut unique = HashSet::new();
        for id in ids {
            let id = id.as_str().filter(|id| !id.is_empty()).ok_or_else(|| {
                "review control state qualifying change finding_ids must contain non-empty strings"
                    .to_owned()
            })?;
            if !unique.insert(id) {
                return Err(
                    "review control state qualifying change finding_ids must be unique".into(),
                );
            }
        }
        if reason == "mandatory_base_integration" && !ids.is_empty() {
            return Err(
                "mandatory base integration must not claim contract/root finding ids".into(),
            );
        }
        if matches!(
            reason,
            "in_scope_contract_root_repair" | external_finding::REASON
        ) && ids.is_empty()
        {
            return Err("contract/root repair must bind at least one finding id".into());
        }
    } else if matches!(
        reason,
        "in_scope_contract_root_repair" | external_finding::REASON
    ) {
        return Err("contract/root repair must bind finding ids".into());
    }
    if reason == external_finding::REASON {
        let source = change.get("external_finding").ok_or_else(|| {
            "authenticated external finding repair must bind its source".to_owned()
        })?;
        let facts = external_finding::check(source)?;
        if string_ids(
            finding_ids.ok_or("missing finding ids")?,
            "qualifying change finding ids",
        )? != facts.finding_ids.iter().cloned().collect()
        {
            return Err("qualifying change finding ids do not bind the external source".into());
        }
    } else if change.contains_key("external_finding") {
        return Err("external finding source requires its typed post-cap reason".into());
    }
    Ok(())
}

fn string_ids(value: &Value, label: &str) -> Result<HashSet<String>, String> {
    let ids = value
        .as_array()
        .ok_or_else(|| format!("{label} must be an array"))?;
    let mut unique = HashSet::new();
    for id in ids {
        let id = id
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or_else(|| format!("{label} must contain non-empty strings"))?;
        if !unique.insert(id.to_owned()) {
            return Err(format!("{label} must be unique"));
        }
    }
    Ok(unique)
}
