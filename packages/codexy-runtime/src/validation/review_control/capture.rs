use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    history::{Blocker, History},
    packet,
    policy::{self, Reviewer},
    presence::RequiredNullable,
    repository, terminal,
};

const PRODUCER_SCHEMA: &str = "codexy.review-control-producer-request.v1";
const OUTPUT_SCHEMA: &str = "codexy.review-control-production.v1";
const TERMINAL_SCHEMA: &str = "codexy.review-terminal-record.v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProducerRequest {
    pub(super) schema: String,
    pub(super) binding: Binding,
    pub(super) terminal_record: TerminalRecord,
    pub(super) packet: Value,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    pub(super) issue_number: u64,
    pub(super) pull_request_number: u64,
    pub(super) base_oid: String,
    pub(super) head_oid: String,
    pub(super) diff_sha256: String,
    pub(super) profile: String,
    pub(super) reviewer: Reviewer,
    pub(super) event_id: String,
    pub(super) predecessor_event_id: RequiredNullable<String>,
    pub(super) issue_contract: Value,
    pub(super) budget: ProducerBudget,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProducerBudget {
    pub(super) full_used: u8,
    pub(super) delta_used: u8,
    pub(super) terminal_used: u8,
    pub(super) terminal_limit: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TerminalRecord {
    pub(super) schema: String,
    pub(super) head_oid: String,
    pub(super) profile: String,
    pub(super) reviewer: Option<Reviewer>,
    pub(super) state: String,
    pub(super) event_id: String,
    pub(super) blockers: Vec<Blocker>,
    pub(super) ledger: History,
}

pub(super) fn build_pr_state(
    plugin_root: &std::path::Path,
    base_text: &str,
    control_text: &str,
) -> Result<Value> {
    let mut state: Value = serde_json::from_str(base_text)?;
    let control: Value = serde_json::from_str(control_text)?;
    let object = state
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("base PR state must be an object"))?;
    if object.contains_key("reviewControl") {
        bail!("base PR state must not already contain review control fields");
    }
    if !control.is_object() {
        bail!("review control state must be an object");
    }
    object.insert("reviewControl".into(), control);
    if let Some(error) = terminal::check_handoff(plugin_root, &state)
        .into_iter()
        .next()
    {
        bail!(error);
    }
    Ok(state)
}

pub(super) fn produce(
    plugin_root: &std::path::Path,
    repository_root: &std::path::Path,
    request_text: &str,
) -> Result<Value> {
    let request: ProducerRequest = serde_json::from_str(request_text)
        .map_err(|error| anyhow::anyhow!("review-control producer request is invalid: {error}"))?;
    if request.schema != PRODUCER_SCHEMA {
        bail!("review-control producer request has an unsupported schema");
    }
    if request.binding.issue_number == 0
        || request.binding.pull_request_number == 0
        || request.binding.profile != "strict"
        || request.binding.budget.terminal_used != 2
        || request.binding.budget.terminal_limit != 3
    {
        bail!("review-control producer binding is not the selected 2/3 strict review");
    }
    let profiles = policy::load(plugin_root)?;
    let profile = profiles
        .get(&request.binding.profile)
        .ok_or_else(|| anyhow::anyhow!("review-control producer profile is unknown"))?;
    if profile.reviewer.as_ref() != Some(&request.binding.reviewer) {
        bail!("review-control producer reviewer is not the selected Sentinel");
    }
    let current = repository::Current::load(repository_root, &request.binding.base_oid)?;
    if current.base_oid != request.binding.base_oid
        || current.head_oid != request.binding.head_oid
        || current.diff_sha256 != request.binding.diff_sha256
    {
        bail!("review-control producer binding is stale or not Git-authentic");
    }
    validate_terminal(&request, &current)?;
    let packet_text = serde_json::to_string(&request.packet)?;
    packet::validate_only(plugin_root, repository_root, &packet_text)?;
    terminal::validate_packet_binding(&request, &current).map_err(anyhow::Error::msg)?;

    let binding = serde_json::to_value(&request.binding)?;
    let ledger = serde_json::to_value(&request.terminal_record.ledger)?;
    let control_state = serde_json::json!({
        "schema":"codexy.review-control-state.v1",
        "profile":request.binding.profile,
        "decision":if request.terminal_record.state == "parent_decision" { "PARENT_DECISION" } else { "APPROVED" },
        "evidence":{
            "schema":"codexy.review-readiness.v1",
            "head_oid":request.binding.head_oid,
            "profile":request.binding.profile,
            "reviewer":request.binding.reviewer,
            "state":request.terminal_record.state,
            "event_id":request.binding.event_id,
            "blockers":request.terminal_record.blockers,
            "binding":binding,
        },
        "ledger":ledger,
    });
    if let Some(error) = terminal::check_handoff(
        plugin_root,
        &serde_json::json!({"headRefOid":request.binding.head_oid,"reviewControl":control_state}),
    )
    .into_iter()
    .next()
    {
        bail!("generated review-control state is invalid: {error}");
    }
    Ok(serde_json::json!({
        "schema":OUTPUT_SCHEMA,
        "binding":binding,
        "packet":request.packet,
        "ledger":ledger,
        "control_state":control_state,
    }))
}

fn validate_terminal(request: &ProducerRequest, current: &repository::Current) -> Result<()> {
    let record = &request.terminal_record;
    if record.schema != TERMINAL_SCHEMA
        || record.head_oid != current.head_oid
        || record.profile != request.binding.profile
        || record.reviewer.as_ref() != Some(&request.binding.reviewer)
        || !matches!(record.state.as_str(), "passed" | "parent_decision")
        || record.event_id != request.binding.event_id
    {
        bail!(
            "review-control producer terminal record is not the selected current-head Sentinel event"
        );
    }
    record.ledger.validate()?;
    let event = record
        .ledger
        .events
        .last()
        .ok_or_else(|| anyhow::anyhow!("review-control producer terminal ledger is empty"))?;
    if event.id != request.binding.event_id
        || event.predecessor_event_id.as_deref() != request.binding.predecessor_event_id.as_deref()
        || event.profile != request.binding.profile
        || event.head_oid != current.head_oid
        || event.base_oid != request.binding.base_oid
        || event.state != record.state
        || event.blockers != record.blockers
        || (event.full_used, event.delta_used)
            != (
                request.binding.budget.full_used,
                request.binding.budget.delta_used,
            )
        || serde_json::to_value(&event.issue_contract)? != request.binding.issue_contract
        || event.issue_contract_sha256 != terminal::digest(&request.binding.issue_contract)
    {
        bail!("review-control producer terminal record does not bind its ledger tip");
    }
    Ok(())
}
