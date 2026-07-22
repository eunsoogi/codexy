use std::process::Output;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

fn assert_rejected(evidence: &str) -> TestResult {
    assert!(!run_ownership_validator(evidence)?.status.success());
    Ok(())
}

fn assert_allowed(evidence: &str) -> TestResult {
    assert!(run_ownership_validator(evidence)?.status.success());
    Ok(())
}

#[test]
fn validator_rejects_external_owner_decision_before_child_setup() -> TestResult {
    assert_rejected(
        r#"Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: external/human-owned implementation lane
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None
Child branch codexy/231-branch-classification-guard was created after classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_allows_passive_negated_child_setup_evidence() -> TestResult {
    assert_allowed(
        r#"Lane ownership: child-owned
Child branch was not created before task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_allows_routing_only_child_delegation_classification_completion() -> TestResult {
    assert_allowed(
        r#"Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: routing-only child delegation to child thread thread-148; parent remains coordination-only
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None
Child branch codexy/231-branch-classification-guard was created after classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_before_the_task_classification_setup_claim() -> TestResult {
    assert_rejected(
        r#"Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: current-thread-owned child implementation lane
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None
Child branch codexy/231-branch-classification-guard was created before the task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_list_style_task_classification_header() -> TestResult {
    assert_rejected(
        r#"Ownership metadata source: parent-supplied
Lane ownership: child-owned
- Task classification:
- Lane type: implementation
- Secondary surfaces: workflow, validators
- Owner decision: current-thread-owned child implementation lane
- Atomic scope: issue-sized
- Required skills: task-classification, codex-orchestration, git-workflow
- Required tools/evidence: goal, plan, codegraph, LSP, Sentinel
- First allowed action: create branch after classification
- Stop/blocker: None
Child branch codexy/231-branch-classification-guard was created after classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_bare_codexy_checkout_before_classification() -> TestResult {
    assert_rejected(
        r#"Lane ownership: child-owned
Child checked out codexy/231-classification-guard before task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_allows_setup_terms_inside_required_tools_metadata() -> TestResult {
    assert_allowed(
        r#"Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: current-thread-owned child implementation lane
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: branch/worktree setup evidence for codexy/231-classification-guard, goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None
Child branch codexy/231-branch-classification-guard was created after classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_setup_before_list_style_classification_without_prior_owner() -> TestResult {
    assert_rejected(
        r#"Child branch codexy/231-branch-classification-guard was created before task classification.
- Task classification:
- Lane type: implementation
- Secondary surfaces: workflow, validators
- Owner decision: current-thread-owned child implementation lane
- Atomic scope: issue-sized
- Required skills: task-classification, codex-orchestration, git-workflow
- Required tools/evidence: goal, plan, codegraph, LSP, Sentinel
- First allowed action: create branch after classification
- Stop/blocker: None
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_allows_negated_setup_terms_inside_required_tools_metadata() -> TestResult {
    assert_allowed(
        r#"Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: current-thread-owned child implementation lane
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: evidence that child branch/worktree setup did not occur before classification, goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None
Child branch codexy/231-branch-classification-guard was created after classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_allows_negative_test_setup_terms_inside_required_tools_metadata() -> TestResult {
    assert_allowed(
        r#"Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: current-thread-owned child implementation lane
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: negative test for child branch/worktree setup before classification, goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None
Child branch codexy/231-branch-classification-guard was created after classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_mixed_negative_test_metadata_and_actual_setup_claim() -> TestResult {
    assert_rejected(
        r#"Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: current-thread-owned child implementation lane
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: negative test for child branch/worktree setup before classification; child branch codexy/231-branch-classification-guard was created before classification
First allowed action: create branch after classification
Stop/blocker: None
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_allows_no_setup_occurred_inside_required_tools_metadata() -> TestResult {
    assert_allowed(
        r#"Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: current-thread-owned child implementation lane
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: evidence that no child branch/worktree setup occurred before classification, goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None
Child branch codexy/231-branch-classification-guard was created after classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}
