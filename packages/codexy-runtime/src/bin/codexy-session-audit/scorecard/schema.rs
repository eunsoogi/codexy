#![allow(dead_code)]

use serde::Deserialize;
use serde_json::Value;

const NULLABLE_MEASURES: [&str; 6] = [
    "inputTokens",
    "wallTimeMs",
    "observedCostUsd",
    "toolInputBytes",
    "toolOutputBytes",
    "cacheInputTokens",
];

pub(super) fn has_required_nullable_fields(value: &Value) -> bool {
    value
        .get("candidate")
        .and_then(Value::as_object)
        .is_some_and(|candidate| candidate.contains_key("installedContentSha256"))
        && value
            .get("comparisons")
            .and_then(Value::as_array)
            .is_some_and(|comparisons| {
                comparisons.iter().all(|comparison| {
                    ["before", "after"].iter().all(|side| {
                        comparison
                            .get(side)
                            .and_then(Value::as_object)
                            .is_some_and(|values| {
                                NULLABLE_MEASURES
                                    .iter()
                                    .all(|name| values.contains_key(*name))
                            })
                    })
                })
            })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Scorecard {
    pub(super) schema: String,
    pub(super) corpus_id: String,
    pub(super) candidate: Candidate,
    pub(super) thresholds: Thresholds,
    pub(super) measure_availability: MeasureAvailability,
    pub(super) comparisons: Vec<Comparison>,
    pub(super) decision_inputs: Vec<DecisionInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Candidate {
    pub(crate) head: String,
    pub(crate) installed_content_sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Thresholds {
    pub(crate) median_input_token_reduction_min_pct: f64,
    pub(crate) p95_tool_output_byte_reduction_min_pct: f64,
    pub(crate) max_p0_p1_misses: u64,
    pub(crate) acceptance_min_pct: f64,
    pub(crate) max_repair_cycle_increase: i64,
    pub(crate) max_review_cycle_increase: i64,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Availability {
    Available,
    Unavailable,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MeasureAvailability {
    pub(crate) input_tokens: Availability,
    pub(crate) wall_time_ms: Availability,
    pub(crate) observed_cost_usd: Availability,
    pub(crate) tool_input_bytes: Availability,
    pub(crate) tool_output_bytes: Availability,
    pub(crate) cache_input_tokens: Availability,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Comparison {
    pub(crate) id: String,
    pub(super) optimization_id: String,
    pub(super) optimization_set: Vec<String>,
    pub(super) model: String,
    pub(super) effort: Effort,
    pub(super) task_id: String,
    pub(super) task_class: TaskClass,
    pub(super) owner: Owner,
    pub(super) phase: Phase,
    pub(super) window: Window,
    pub(crate) before: Measurements,
    pub(crate) after: Measurements,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Effort {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
    Ultra,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub(super) enum TaskClass {
    SimpleDiscovery,
    GeneralImplementation,
    ReviewResponse,
    StrictWork,
    CompactionRecovery,
}

impl TaskClass {
    pub(super) fn for_task_id(task_id: &str) -> Option<Self> {
        match task_id {
            "simple-discovery" => Some(Self::SimpleDiscovery),
            "general-implementation" => Some(Self::GeneralImplementation),
            "review-response" => Some(Self::ReviewResponse),
            "strict-work" => Some(Self::StrictWork),
            "compaction-recovery" => Some(Self::CompactionRecovery),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Owner {
    pub(super) kind: OwnerKind,
    pub(super) id: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub(super) enum OwnerKind {
    Root,
    Child,
    SelectedReviewer,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Phase {
    Discovery,
    Implementation,
    ReviewResponse,
    StrictWork,
    CompactionRecovery,
    Wait,
    ToolOutput,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum Window {
    EqualRecordCount {
        #[serde(rename = "beforeRecords")]
        before: u64,
        #[serde(rename = "afterRecords")]
        after: u64,
    },
    EqualDuration {
        #[serde(rename = "beforeDurationSeconds")]
        before: u64,
        #[serde(rename = "afterDurationSeconds")]
        after: u64,
    },
}

impl Window {
    pub(super) const fn values(&self) -> (u64, u64, &'static str) {
        match self {
            Self::EqualRecordCount { before, after } => (*before, *after, "equal record-count"),
            Self::EqualDuration { before, after } => (*before, *after, "equal-duration"),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Measurements {
    pub(crate) acceptance_runs: u64,
    pub(crate) accepted_runs: u64,
    pub(crate) p0_p1_misses: u64,
    pub(crate) proof_complete_runs: u64,
    pub(crate) repairs: u64,
    pub(crate) review_cycles: u64,
    pub(crate) input_tokens: Option<u64>,
    pub(super) wall_time_ms: Option<u64>,
    pub(super) observed_cost_usd: Option<f64>,
    pub(super) tool_input_bytes: Option<u64>,
    pub(crate) tool_output_bytes: Option<u64>,
    pub(super) cache_input_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DecisionInput {
    pub(super) optimization_id: String,
    pub(super) comparison_ids: Vec<String>,
    pub(super) evidence_state: EvidenceState,
    pub(super) unavailable_measures: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum EvidenceState {
    Observable,
    Unobservable,
}
