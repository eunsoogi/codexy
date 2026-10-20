use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::Value;

use super::{Predicates, routing_json};

const LEGACY_REQUEST_SCHEMA: &str = "codexy.child-routing-request.v1";
const RECIPIENT_REQUEST_SCHEMA: &str = "codexy.child-routing-request.v2";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ThreadCapabilities {
    models: Vec<ModelCapability>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    pub(super) schema: String,
    pub(super) classification: String,
    #[serde(default)]
    pub(super) named_specialist: Option<String>,
    #[serde(default)]
    pub(super) simple_predicates: Option<Predicates>,
    pub(super) codex_thread_operation: String,
    #[serde(default)]
    pub(super) codex_thread_direction: Option<ThreadDirection>,
    #[serde(default)]
    pub(super) codex_thread_capabilities: Option<ThreadCapabilities>,
    #[serde(default)]
    pub(super) recipient_assignment: Option<RecipientAssignment>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RecipientAssignment {
    pub(super) model: String,
    pub(super) thinking: String,
}

#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) enum ThreadDirection {
    #[serde(rename = "parent_to_generic")]
    OrchestratorToWorker,
    #[serde(rename = "child_to_root")]
    WorkerToOrchestrator,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelCapability {
    model: String,
    thinking: Vec<String>,
}

pub(super) fn validate_operation(operation: &str) -> Result<()> {
    if matches!(operation, "create_thread" | "send_message_to_thread") {
        Ok(())
    } else {
        bail!("Worker routing request names an unsupported Codex thread operation")
    }
}

pub(super) fn parse_request(text: &str) -> Result<Request> {
    let value = routing_json::parse(text).map_err(anyhow::Error::msg)?;
    reject_sender_settings(&value)?;
    let request = serde_json::from_value::<Request>(value)?;
    validate_operation(&request.codex_thread_operation)?;
    let parent_follow_up = matches!(
        (
            &*request.codex_thread_operation,
            request.codex_thread_direction
        ),
        (
            "send_message_to_thread",
            Some(ThreadDirection::OrchestratorToWorker)
        )
    );
    // V1 remains valid for unaffected routes; only v2 parent follow-ups carry recipient assignments.
    match request.schema.as_str() {
        LEGACY_REQUEST_SCHEMA if request.recipient_assignment.is_some() => {
            bail!("Version 1 does not accept recipient_assignment; use schema v2")
        }
        LEGACY_REQUEST_SCHEMA => {}
        RECIPIENT_REQUEST_SCHEMA if parent_follow_up => {}
        RECIPIENT_REQUEST_SCHEMA => {
            bail!("Version 2 is reserved for recipient-bound parent follow-ups")
        }
        _ => bail!("Worker routing request has an unsupported schema"),
    }
    match (
        request.codex_thread_operation.as_str(),
        request.codex_thread_direction,
    ) {
        ("create_thread", None)
        | ("send_message_to_thread", Some(ThreadDirection::OrchestratorToWorker))
        | ("send_message_to_thread", Some(ThreadDirection::WorkerToOrchestrator)) => {}
        _ => bail!("Worker routing request has an invalid thread delivery direction"),
    }
    Ok(request)
}

fn reject_sender_settings(value: &Value) -> Result<()> {
    let Some(object) = value.as_object() else {
        return Ok(());
    };
    if ["model", "thinking", "sender_model", "sender_thinking"]
        .iter()
        .any(|field| object.contains_key(*field))
    {
        bail!("Worker routing request contains sender settings; use recipient_assignment")
    }
    Ok(())
}

pub(super) fn supports(
    capabilities: Option<&ThreadCapabilities>,
    model: &str,
    thinking: &str,
) -> bool {
    // A configured route is supported only when both the exact model and requested thinking value are advertised.
    capabilities.is_some_and(|capabilities| {
        capabilities.models.iter().any(|candidate| {
            candidate.model == model && candidate.thinking.iter().any(|value| value == thinking)
        })
    })
}
