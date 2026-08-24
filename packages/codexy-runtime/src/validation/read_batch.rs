pub mod bounds;
pub mod identity;
pub mod references;

use std::collections::BTreeSet;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

pub use self::references::ReadReference;

pub const MAX_OPERATIONS: usize = 4;

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadBatchPlan {
    pub operations: Vec<ReadOperation>,
    pub aggregate_output_bound: u64,
    pub outcomes: Vec<ReadOutcome>,
    pub measurements: Measurements,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadOperation {
    pub id: String,
    pub reference: ReadReference,
    pub output_bound: u64,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadOutcome {
    pub id: String,
    pub output_bytes: u64,
    pub attempts: u64,
    pub status: OutcomeStatus,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutcomeStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Measurements {
    pub input_tokens: InputTokens,
    pub output_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct InputTokens {
    pub value: u64,
}

impl ReadBatchPlan {
    /// Checks the bounded, independent, read-only batch contract.
    pub fn validate(&self) -> Result<()> {
        if self.operations.is_empty() || self.operations.len() > MAX_OPERATIONS {
            bail!("read batch must contain one to four operations");
        }
        let mut operation_ids = BTreeSet::new();
        let mut aggregate = 0_u64;
        for operation in &self.operations {
            if operation.id.trim().is_empty() || !operation_ids.insert(&operation.id) {
                bail!("operation identities must be non-empty and unique");
            }
            if !operation.reference.is_eligible() {
                bail!("every operation must be independent, read-only, and bounded");
            }
            if operation.output_bound == 0 {
                bail!("every operation needs a positive output bound");
            }
            aggregate = aggregate
                .checked_add(operation.output_bound)
                .ok_or_else(|| anyhow::anyhow!("aggregate output bound overflows u64"))?;
        }
        if self.aggregate_output_bound < aggregate {
            bail!("aggregate output bound must cover every operation");
        }
        for outcome in &self.outcomes {
            if !operation_ids.contains(&outcome.id) {
                bail!("outcome identity is not present in operations");
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn successful_outcomes(&self) -> Vec<&ReadOutcome> {
        self.outcomes
            .iter()
            .filter(|outcome| outcome.status == OutcomeStatus::Success)
            .collect()
    }
}
