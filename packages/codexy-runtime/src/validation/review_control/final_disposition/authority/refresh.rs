use std::collections::HashSet;

use serde_json::{Value, json};

use super::super::fields::{finding_ids, oid, text};
use super::{CAPTURE_METHOD, SCHEMA, body, source};

struct Facts {
    source_head: String,
    current_head: String,
    base: String,
    event_id: String,
    finding_ids: HashSet<String>,
}

pub(super) fn refresh(
    control: &mut Value,
    locator: Option<&Value>,
    current: &Value,
) -> Result<(), String> {
    let facts = facts(control)?;
    let locator = locator
        .cloned()
        .or_else(|| existing_locator(control).cloned())
        .ok_or_else(|| "final disposition requires an authenticated locator".to_owned())?;
    super::super::super::post_cap_disposition::validate_locator(&locator, current)?;
    let (ci, maintainer) = super::super::super::post_cap_disposition::read_final_sources(
        &locator,
        Some(&facts.current_head),
    )?;
    let repository = current
        .get("repository")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "final disposition current PR snapshot must contain repository".to_owned()
        })?;
    let owning_issue = control
        .get("issue_number")
        .and_then(Value::as_u64)
        .ok_or_else(|| "final disposition control must contain issue_number".to_owned())?;
    let pull_request = current
        .get("number")
        .and_then(Value::as_u64)
        .ok_or_else(|| "final disposition current PR snapshot must contain number".to_owned())?;
    if current.get("baseRefOid").and_then(Value::as_str) != Some(&facts.base)
        || current.get("headRefOid").and_then(Value::as_str) != Some(&facts.current_head)
    {
        return Err(
            "final disposition current PR snapshot disagrees with its bound head or base".into(),
        );
    }
    let expected = body::Expected {
        repository,
        owning_issue,
        pull_request,
        base: &facts.base,
        source_head: &facts.source_head,
        current_head: &facts.current_head,
        review_event_id: &facts.event_id,
        finding_ids: &facts.finding_ids,
    };
    let maintainer = source::project(&maintainer, &locator, &expected)?;
    let authority = json!({
        "schema": SCHEMA,
        "locator": locator,
        "repository": repository,
        "owningIssue": {"repository": repository, "number": owning_issue},
        "pullRequest": {
            "repository": repository,
            "number": pull_request,
            "baseRefOid": facts.base,
            "headRefOid": facts.current_head
        },
        "sources": {
            "currentHeadCi": ci,
            "maintainerDecision": maintainer
        },
        "capture": {
            "provider": "github",
            "method": CAPTURE_METHOD,
            "authenticated": true
        }
    });
    let disposition = control
        .get_mut("final_disposition")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "final disposition must be an object".to_owned())?;
    if let Some(existing) = disposition.get("authority") {
        let locator_only = existing
            .as_object()
            .is_some_and(|object| object.len() == 1 && object.contains_key("locator"));
        if !locator_only && existing != &authority {
            return Err("final disposition authority changed after live refresh".into());
        }
        if locator_only {
            disposition.insert("authority".into(), authority);
        }
    } else {
        disposition.insert("authority".into(), authority);
    }
    Ok(())
}

fn facts(control: &Value) -> Result<Facts, String> {
    let disposition = control
        .get("final_disposition")
        .and_then(Value::as_object)
        .ok_or_else(|| "final disposition must be an object".to_owned())?;
    let history = control
        .get("terminal_review_history")
        .and_then(Value::as_array)
        .ok_or_else(|| "final disposition requires terminal review history".to_owned())?;
    let third = history
        .get(2)
        .and_then(Value::as_object)
        .ok_or_else(|| "final disposition requires a third review event".to_owned())?;
    Ok(Facts {
        source_head: text(disposition, "source_head", "final disposition")?.to_owned(),
        current_head: oid(disposition, "head_oid", "final disposition")?.to_owned(),
        base: oid(disposition, "base_oid", "final disposition")?.to_owned(),
        event_id: text(third, "id", "third review event")?.to_owned(),
        finding_ids: finding_ids(
            third
                .get("unresolved_findings")
                .and_then(Value::as_array)
                .ok_or_else(|| "third review event must list findings".to_owned())?,
            "third-review findings",
        )?,
    })
}

fn existing_locator(control: &Value) -> Option<&Value> {
    control
        .get("final_disposition")?
        .get("authority")?
        .get("locator")
}
