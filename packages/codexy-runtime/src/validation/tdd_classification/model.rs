use serde::Deserialize;

pub(super) const V1_REQUEST_SCHEMA: &str = "codexy.tdd-classification-request.v1";
pub(super) const V2_REQUEST_SCHEMA: &str = "codexy.tdd-classification-request.v2";
pub(super) const V2_RESULT_SCHEMA: &str = "codexy.tdd-classification-result.v2";

pub(super) const ENGINEERING: [&str; 14] = [
    "production_code",
    "runtime_behavior",
    "validator",
    "parser",
    "markdown_backed_parser",
    "hook",
    "cli",
    "workflow",
    "installer",
    "package_resolution",
    "tool_behavior",
    "defect_repair",
    "behavior_preserving_refactor",
    "executable_contract",
];
pub(super) const V2_ENGINEERING: [&str; 12] = [
    "production_code",
    "runtime_behavior",
    "validator",
    "parser",
    "markdown_backed_parser",
    "hook",
    "cli",
    "workflow",
    "installer",
    "package_resolution",
    "tool_behavior",
    "executable_contract",
];
pub(super) const NON_ENGINEERING: [&str; 11] = [
    "readme",
    "documentation",
    "instruction_only_skill",
    "agent_prompt",
    "declarative_metadata",
    "issue_or_pr_metadata",
    "roadmap_or_release_prose",
    "inventory",
    "diagram",
    "example",
    "copy_edit",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LegacyRequest {
    pub(super) schema: String,
    pub(super) boundaries: Vec<String>,
}

#[derive(Clone, Copy, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub(super) enum ChangePurpose {
    Feature,
    DefectRepair,
    BehaviorPreservingRefactor,
    InstructionOnly,
}

#[derive(Clone, Copy, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub(super) enum Risk {
    Permission,
    DestructiveState,
    Recovery,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ReproductionStatus {
    Available,
    Unavailable,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reproduction {
    pub(super) status: ReproductionStatus,
    #[serde(default)]
    pub(super) reason: Option<String>,
    #[serde(default)]
    pub(super) alternative: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Change {
    pub(super) purpose: ChangePurpose,
    #[serde(default)]
    pub(super) reproduction: Option<Reproduction>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BoundaryRequest {
    pub(super) id: String,
    pub(super) kind: String,
    pub(super) change: Change,
    pub(super) risks: Vec<Risk>,
    pub(super) test_first_required: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct V2Request {
    pub(super) schema: String,
    pub(super) boundaries: Vec<BoundaryRequest>,
}

pub(super) enum Request {
    V1(LegacyRequest),
    V2(V2Request),
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum TddMode {
    Required,
    Optional,
    NotApplicable,
}

impl TddMode {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Optional => "optional",
            Self::NotApplicable => "not_applicable",
        }
    }
}
