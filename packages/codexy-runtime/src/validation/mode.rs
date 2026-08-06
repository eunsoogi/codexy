#[derive(Debug, Clone)]
pub enum Mode {
    All,
    InstructionPolicy,
    OrchestrationRouting,
    Lsp,
    RustLspReadiness,
    MergeMessage {
        expected_issue: Option<u64>,
        expected_pr: Option<u64>,
        message: String,
    },
    MergeAuthorization {
        authorization: String,
        pr_state: String,
    },
    PrTitle {
        title: String,
    },
    IssueTitle {
        title: String,
    },
    PrLabels {
        pr_state: String,
    },
    IssueIntake {
        receipt: String,
    },
    CompletionHandoff {
        handoff: String,
        pr_state: String,
    },
    ReviewResponseCluster(String),
    Mcp,
    Hooks,
    Roles,
    RuntimeArtifacts,
    ChildLaneOwnership {
        evidence: String,
    },
    TouchedLoc {
        base_ref: String,
    },
}
