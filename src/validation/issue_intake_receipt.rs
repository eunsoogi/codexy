use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct Candidate {
    pub(super) parent_approval: ParentApproval,
    pub(super) classification: Classification,
    pub(super) reproduction: Reproduction,
    pub(super) ownership: Ownership,
    pub(super) duplicate_search: DuplicateSearch,
    pub(super) necessity: Necessity,
    pub(super) title: String,
    pub(super) body: String,
    pub(super) labels: Vec<String>,
    pub(super) repository_labels: Vec<String>,
    pub(super) repository_milestones: Vec<String>,
    pub(super) repository_assignees: Vec<String>,
    pub(super) milestone: String,
    pub(super) assignee: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum ApprovalDecision {
    Approved,
    Rejected,
}

#[derive(Debug, Deserialize)]
pub(super) struct ParentApproval {
    pub(super) decision: ApprovalDecision,
    pub(super) source_task_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Classification {
    IssueSizedDefect,
    UnsupportedSynthetic,
    SameClassObservation,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum EvidenceDecision {
    Supported,
    Unsupported,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum SurfaceKind {
    RealProducer,
    UserFacing,
}

#[derive(Debug, Deserialize)]
pub(super) struct Reproduction {
    pub(super) decision: EvidenceDecision,
    #[serde(rename = "surface_kind")]
    _surface_kind: SurfaceKind,
    pub(super) surface: String,
    pub(super) steps: Vec<String>,
    pub(super) observed: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum OwnershipDecision {
    CannotOwn,
    CanOwn,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum OwnerKind {
    Issue,
    PullRequest,
}

#[derive(Debug, Deserialize)]
pub(super) struct ExistingOwner {
    #[serde(rename = "kind")]
    _kind: OwnerKind,
    pub(super) number: u64,
}

#[derive(Debug, Deserialize)]
pub(super) struct Ownership {
    pub(super) decision: OwnershipDecision,
    pub(super) existing_owner: ExistingOwner,
    pub(super) rationale: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(super) enum IssueState {
    Open,
    Closed,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum MatchKind {
    Exact,
    Related,
}

#[derive(Debug, Deserialize)]
pub(super) struct SearchResult {
    pub(super) issue: u64,
    #[serde(rename = "state")]
    _state: IssueState,
    pub(super) match_kind: MatchKind,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub(super) enum DuplicateConclusion {
    NoDuplicate,
    Duplicate { canonical_issue: u64 },
}

#[derive(Debug, Deserialize)]
pub(super) struct DuplicateSearch {
    pub(super) states: Vec<IssueState>,
    pub(super) search_terms: Vec<String>,
    pub(super) results: Vec<SearchResult>,
    pub(super) conclusion: DuplicateConclusion,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum NecessityDecision {
    ThinHarnessChangeRequired,
    NoChange,
}

#[derive(Debug, Deserialize)]
pub(super) struct Necessity {
    pub(super) decision: NecessityDecision,
    pub(super) rationale: String,
}
