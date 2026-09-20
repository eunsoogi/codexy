use serde::Serialize;

use super::super::change_impact::{ImpactLevel, UnknownArea};
use super::super::change_input::ObservedState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckDefinition {
    pub id: String,
    pub command: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingOwner {
    User,
    Repository,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingKind {
    Path,
    SharedConfiguration,
    Fixture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckMapping {
    pub owner: MappingOwner,
    pub kind: MappingKind,
    pub pattern: String,
    pub check_ids: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CheckMappings {
    pub checks: Vec<CheckDefinition>,
    pub mappings: Vec<CheckMapping>,
}

impl CheckMappings {
    #[must_use]
    pub const fn new(checks: Vec<CheckDefinition>, mappings: Vec<CheckMapping>) -> Self {
        Self { checks, mappings }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeArea {
    Documentation,
    Module,
    SharedFixture,
    Lockfile,
    Configuration,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum DependencyState {
    Confirmed,
    Unconfirmed { detail: String },
}

impl Default for DependencyState {
    fn default() -> Self {
        Self::Unconfirmed {
            detail: "dependency state was not confirmed".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SelectionOptions {
    pub dependency_state: DependencyState,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GapReason {
    MissingMapping,
    EmptyMapping,
    MissingCheck,
    MissingCommand,
    UnknownImpact,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct SelectionGap {
    pub path: Option<String>,
    pub area: Option<ChangeArea>,
    pub mapping_pattern: Option<String>,
    pub reason: GapReason,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BroaderReason {
    SharedFixture,
    Lockfile,
    Configuration,
    PartialImpact,
    UnknownImpact,
    UnconfirmedDependencies,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct BroaderVerification {
    pub reason: BroaderReason,
    pub paths: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManualReason {
    MissingMapping,
    MissingCheck,
    PartialAnalysis,
    UnknownImpact,
    UnconfirmedDependencies,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ManualJudgment {
    pub reason: ManualReason,
    pub paths: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckRecommendation {
    pub id: String,
    pub command: String,
    pub description: String,
    pub paths: Vec<String>,
    pub areas: Vec<ChangeArea>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckSelection {
    pub baseline_revision: String,
    pub observed_state: ObservedState,
    pub recommendations: Vec<CheckRecommendation>,
    pub gaps: Vec<SelectionGap>,
    pub broader_verification: Vec<BroaderVerification>,
    pub manual_judgment: Vec<ManualJudgment>,
    pub impact_unknown: Vec<UnknownArea>,
    pub partial: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PathEvidence {
    pub(super) area: ChangeArea,
    pub(super) impact: ImpactLevel,
}
