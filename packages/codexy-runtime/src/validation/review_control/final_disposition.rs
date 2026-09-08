use std::{collections::HashSet, path::Path};

use serde_json::{Map, Value};

mod authority;
mod fields;
mod repository;
mod summaries;
mod transition;

use fields::{finding_ids, oid, reject_unknown, section, section_ids, text};

pub(super) const SCHEMA: &str = "codexy.review-control-final-disposition.v1";

#[allow(clippy::too_many_arguments)]
pub(super) fn check(
    value: &Value,
    history: &[Value],
    reviewed_head: &str,
    terminal: &str,
    findings: &[Value],
    current_head: &str,
    base_oid: Option<&str>,
    issue_number: u64,
    repository: Option<&str>,
    pull_request: Option<u64>,
    pr_state: Option<&Value>,
) -> Result<(), String> {
    let disposition = value
        .as_object()
        .ok_or_else(|| "review control final_disposition must be an object".to_owned())?;
    reject_unknown(
        disposition,
        &[
            "schema",
            "kind",
            "review_event_id",
            "source_head",
            "head_oid",
            "base_oid",
            "addressed_finding_ids",
            "remaining_finding_ids",
            "source_repair",
            "evidence_refresh",
            "authority",
        ],
        "final_disposition",
    )?;
    if text(disposition, "schema", "final disposition")? != SCHEMA
        || text(disposition, "kind", "final disposition")? != "third_block_repair"
    {
        return Err("review control final disposition has an unsupported schema or kind".into());
    }
    if history.len() != 3 {
        return Err("review control final disposition requires exactly three review events".into());
    }
    let third = history[2].as_object().ok_or_else(|| {
        "review control final disposition requires a third review event".to_owned()
    })?;
    if text(third, "terminal_result", "third review event")? != "BLOCK" || terminal != "BLOCK" {
        return Err("review control final disposition requires the authentic third BLOCK".into());
    }
    if text(disposition, "review_event_id", "final disposition")?
        != text(third, "id", "third review event")?
        || text(disposition, "source_head", "final disposition")?
            != text(third, "reviewed_head", "third review event")?
    {
        return Err("review control final disposition does not bind the third review event".into());
    }
    let source_head = text(third, "reviewed_head", "third review event")?;
    if reviewed_head != source_head {
        return Err("final disposition must preserve the reviewer projection".into());
    }
    if oid(disposition, "head_oid", "final disposition")? != current_head {
        return Err(
            "review control final disposition does not bind the current head or base".into(),
        );
    }
    let disposition_base = oid(disposition, "base_oid", "final disposition")?;
    if base_oid.is_some_and(|base| base != disposition_base) {
        return Err(
            "review control final disposition does not bind the current head or base".into(),
        );
    }
    let third_findings = third
        .get("unresolved_findings")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| {
            "review control final disposition requires third-review findings".to_owned()
        })?;
    if findings != third_findings {
        return Err("final disposition must preserve the reviewer findings projection".into());
    }
    let expected_ids = finding_ids(third_findings, "third-review findings")?;
    let addressed =
        section_ids_from_field(disposition, "addressed_finding_ids", "final disposition")?;
    let remaining =
        section_ids_from_field(disposition, "remaining_finding_ids", "final disposition")?;
    if addressed != expected_ids || !remaining.is_empty() {
        return Err("final disposition does not prove all third-review findings addressed".into());
    }
    let source_repair = disposition
        .get("source_repair")
        .map(|value| section(value, "source repair"))
        .transpose()?;
    let evidence_refresh = disposition
        .get("evidence_refresh")
        .map(|value| section(value, "evidence refresh"))
        .transpose()?;
    if source_repair.is_none() && evidence_refresh.is_none() {
        return Err(
            "review control final disposition must record repair or evidence refresh".into(),
        );
    }
    if let Some(repair) = source_repair {
        if text(repair, "from_head", "source repair")?
            != text(third, "reviewed_head", "third review event")?
        {
            return Err("final disposition source repair has a stale source head".into());
        }
        if oid(repair, "evidence_commit", "source repair")?
            == text(third, "reviewed_head", "third review event")?
        {
            return Err("final disposition source repair must advance from the third head".into());
        }
    }
    let source_ids = source_repair
        .map(|repair| section_ids(repair, "source repair"))
        .transpose()?;
    let refresh_ids = evidence_refresh
        .map(|refresh| section_ids(refresh, "evidence refresh"))
        .transpose()?;
    if source_ids.is_none() && refresh_ids.is_none() {
        return Err("final disposition must cover at least one third-review finding".into());
    }
    let mut covered = HashSet::new();
    for id in source_ids
        .iter()
        .flatten()
        .chain(refresh_ids.iter().flatten())
    {
        if !expected_ids.contains(id) || !covered.insert(id.as_str()) {
            return Err("final disposition finding coverage is not an exact partition".into());
        }
    }
    if covered.len() != expected_ids.len() {
        return Err("final disposition does not cover every third-review finding".into());
    }
    if let Some(refresh) = evidence_refresh {
        if text(refresh, "proof_head", "evidence refresh")? != current_head {
            return Err("final disposition evidence refresh is bound to a stale head".into());
        }
        summaries::check(refresh, refresh_ids.as_deref().unwrap_or_default())?;
    }
    if current_head == source_head {
        if source_repair.is_some() {
            return Err("same-head final disposition must be evidence-only".into());
        }
    } else if source_repair.is_none() {
        return Err("a repaired final head requires source repair evidence".into());
    }
    authority::check(
        disposition,
        history,
        source_head,
        current_head,
        disposition_base,
        issue_number,
        repository,
        pull_request,
        pr_state,
    )?;
    Ok(())
}

pub(super) fn refresh_live(
    control: &mut Value,
    locator: Option<&Value>,
    current: &Value,
) -> Result<(), String> {
    authority::refresh(control, locator, current)
}

pub(super) fn check_handoff_state(
    state: &Value,
    control: &Map<String, Value>,
) -> Result<(), String> {
    authority::check_handoff_state(state, control)
}

pub(super) fn check_handoff_repository(
    repository_root: &Path,
    state: &Value,
    control: &Map<String, Value>,
) -> Result<(), String> {
    repository::check_handoff(repository_root, state, control)
}

fn section_ids_from_field(
    object: &Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<HashSet<String>, String> {
    let values = object
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label} must list {key}"))?;
    let mut ids = HashSet::new();
    for value in values {
        let id = value
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or_else(|| format!("{label} {key} must contain non-empty strings"))?;
        if !ids.insert(id.to_owned()) {
            return Err(format!("{label} {key} must be unique"));
        }
    }
    Ok(ids)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn check_transition(
    repository_root: &Path,
    previous: &Value,
    current: &Value,
    previous_control: &Map<String, Value>,
    current_control: &Map<String, Value>,
    previous_count: u64,
    current_count: u64,
    previous_history: &[Value],
    current_history: &[Value],
) -> Result<bool, String> {
    transition::check(
        repository_root,
        previous,
        current,
        previous_control,
        current_control,
        previous_count,
        current_count,
        previous_history,
        current_history,
    )
}
