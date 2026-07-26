#![allow(dead_code)]

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Receipt {
    #[serde(rename = "schemaVersion")]
    pub(super) schema_version: u64,
    pub(super) metadata_only: bool,
    pub(super) lane: Lane,
    pub(super) installed: Installed,
    pub(super) audit: Audit,
    pub(super) metrics: Metrics,
    pub(super) goal_plan_receipts: Vec<String>,
    pub(super) helpers: Vec<Helper>,
    pub(super) command_receipts: Vec<CommandReceipt>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Lane {
    pub(super) issue: String,
    pub(super) pr: String,
    pub(super) branch: String,
    pub(super) head: String,
    pub(super) base: String,
    pub(super) owner_thread_id: String,
    pub(super) child_created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Installed {
    pub(super) plugin_id: String,
    pub(super) version: String,
    pub(super) cache_root_relative: String,
    pub(super) manifest_sha256: String,
    pub(super) changed_files: Vec<ChangedFile>,
    pub(super) content_equivalent: bool,
    pub(super) content_proof: ContentProof,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ContentProof {
    pub(super) source_manifest_sha256: String,
    pub(super) installed_manifest_sha256: String,
    pub(super) source_changed_files: Vec<ChangedFile>,
    pub(super) installed_changed_files: Vec<ChangedFile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CommandReceipt {
    pub(super) command_id: String,
    pub(super) arguments_redacted: bool,
    #[serde(default)]
    pub(super) command: Option<String>,
    pub(super) exit_code: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Audit {
    pub(super) input_sha256: String,
    pub(super) observational_only: bool,
    pub(super) comparison: Comparison,
    pub(super) owner_tree_sessions: Vec<OwnerSession>,
    pub(super) owner_tree_totals: Totals,
    pub(super) duplicate_events_skipped: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Comparison {
    pub(super) owner_boundary: OwnerBoundary,
    pub(super) window_policy: WindowPolicy,
    pub(super) before: Observation,
    pub(super) after: Observation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct OwnerBoundary {
    pub(super) kind: String,
    pub(super) owner_thread_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum WindowPolicy {
    EqualRecordCount {
        #[serde(rename = "beforeRecords")]
        before: u64,
        #[serde(rename = "afterRecords")]
        after: u64,
        comparable: bool,
    },
    EqualDuration {
        #[serde(rename = "beforeDurationSeconds")]
        before: u64,
        #[serde(rename = "afterDurationSeconds")]
        after: u64,
        comparable: bool,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Observation {
    pub(super) session_id: String,
    pub(super) input_sha256: String,
    pub(super) window: Window,
    pub(super) latest_cumulative_tokens: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Window {
    pub(super) records_observed: u64,
    pub(super) turn_events: u64,
    #[serde(default)]
    pub(super) duration_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct OwnerSession {
    pub(super) session_id: String,
    pub(super) owner_root_thread_id: String,
    pub(super) input_sha256: String,
    pub(super) records_observed: u64,
    pub(super) turn_events: u64,
    pub(super) cumulative_tokens: u64,
    pub(super) tool_input_bytes: u64,
    pub(super) tool_output_bytes: u64,
    pub(super) exec_family: Family,
    pub(super) wait_family: Family,
}

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Totals {
    pub(super) session_count: u64,
    pub(super) records_observed: u64,
    pub(super) turn_events: u64,
    pub(super) cumulative_tokens: u64,
    pub(super) tool_input_bytes: u64,
    pub(super) tool_output_bytes: u64,
    pub(super) exec_family: Family,
    pub(super) wait_family: Family,
}

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Family {
    pub(super) calls: u64,
    pub(super) input_bytes: u64,
    pub(super) output_bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ChangedFile {
    pub(super) path: String,
    pub(super) sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Metrics {
    pub(super) review_feedback: u64,
    pub(super) child_age_seconds: u64,
    pub(super) retries_by_kind: Retries,
    pub(super) event_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Retries {
    pub(super) parent_message: u64,
    pub(super) ci: u64,
    pub(super) sentinel: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Helper {
    pub(super) id: String,
    pub(super) owned_slice: String,
}
