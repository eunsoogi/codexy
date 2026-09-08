use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(super) const EVENT_KINDS: &[&str] = &[
    "terminal",
    "failure",
    "drift",
    "missing_delivery",
    "gate_ready",
    "unavailable",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Session {
    pub(super) schema: u8,
    pub(super) session_id: String,
    pub(super) assignment_id: String,
    pub(super) parent: Value,
    pub(super) watcher: Value,
    pub(super) targets: Vec<Value>,
    pub(super) generation: u64,
    pub(super) created_at_ms: u64,
    pub(super) expires_at_ms: u64,
    pub(super) parent_token_hash: String,
    pub(super) watcher_token_hash: String,
    pub(super) next_sequence: u64,
    pub(super) status: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Health {
    pub(super) last_observation_at_ms: Option<u64>,
    pub(super) last_material_event_at_ms: Option<u64>,
    pub(super) watcher_state: String,
    pub(super) last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Event {
    pub(super) event_id: String,
    pub(super) sequence: u64,
    pub(super) kind: String,
    pub(super) target: Value,
    pub(super) summary: String,
    pub(super) observed_at_ms: u64,
    pub(super) evidence: Vec<String>,
    pub(super) fingerprint: String,
}
