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
fn validator_rejects_routing_only_child_delegation_setup_before_classification() -> TestResult {
    assert_rejected(
        r#"Owner decision: routing-only child delegation to child thread thread-148; parent remains coordination-only
Child branch codexy/231-branch-classification-guard was created before task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_allows_parent_owned_owner_decision_setup_before_later_child_lane() -> TestResult {
    assert_allowed(&format!(
        "{}\nBranch codexy/parent-owned was created after classification.\n{}\n{}\nChild branch codexy/231-branch-classification-guard was created after classification.\n{}",
        complete_parent_owned_owner_decision_classification(),
        ownership_footer(),
        complete_child_classification(),
        ownership_footer()
    ))
}

#[test]
fn validator_rejects_child_setup_when_only_prior_lane_was_classified() -> TestResult {
    assert_rejected(&format!(
        "PR: #1\n{}\nChild branch codexy/one was created after classification.\n{}\nPR: #2\nLane ownership: child-owned\nChild branch codexy/two was created immediately after thread rename.\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n",
        complete_child_classification(),
        ownership_footer()
    ))
}

#[test]
fn validator_rejects_list_style_child_setup_when_only_prior_lane_was_classified() -> TestResult {
    assert_rejected(&format!(
        "PR: #1\n{}\nChild branch codexy/one was created after classification.\n{}\nPR: #2\n- Lane ownership: child-owned\nChild branch codexy/two was created immediately after thread rename.\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n",
        complete_child_classification(),
        ownership_footer()
    ))
}

#[test]
fn validator_rejects_list_style_child_lane_boundary_after_prior_classification() -> TestResult {
    assert_rejected(&format!(
        "{}\nChild branch codexy/one was created after classification.\n{}\n- Lane ownership: child-owned\nChild branch codexy/two was created immediately after thread rename.\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n",
        complete_child_classification(),
        ownership_footer()
    ))
}

#[test]
fn validator_allows_neutral_setup_after_previous_child_lane_boundary() -> TestResult {
    assert_allowed(&format!(
        "PR: #1\n{}\nChild branch codexy/one was created after classification.\n{}\nPR: #2\nBranch codexy/neutral-lane was created after classification.\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n",
        complete_child_classification(),
        ownership_footer()
    ))
}

#[test]
fn validator_rejects_child_setup_when_only_prior_lane_precedes_owner_decision() -> TestResult {
    assert_rejected(&format!(
        "{}\nChild branch codexy/one was created after classification.\n{}\nOwner decision: child-owned implementation lane\nChild branch codexy/two was created immediately after thread rename.\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n",
        complete_child_classification(),
        ownership_footer()
    ))
}

#[test]
fn validator_allows_setup_before_next_lane_boundary() -> TestResult {
    assert_allowed(
        r#"PR: #1
Branch codexy/neutral-lane was created after classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
PR: #2
Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: child-owned implementation lane
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
fn validator_allows_setup_before_next_lane_ownership_boundary() -> TestResult {
    assert_allowed(
        r#"Branch codexy/neutral-lane was created after classification.
Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: child-owned implementation lane
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
fn validator_allows_setup_before_next_owner_decision_boundary() -> TestResult {
    assert_allowed(
        r#"Branch codexy/neutral-lane was created after classification.
Owner decision: child-owned implementation lane
Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: child-owned implementation lane
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
fn validator_rejects_setup_before_same_classification_owner_decision() -> TestResult {
    assert_rejected(
        r#"Child branch codexy/231-branch-classification-guard was created before task classification.
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: current-thread-owned child implementation lane
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_allows_owner_metadata_after_complete_child_classification() -> TestResult {
    for owner_metadata in ["Owner: child-thread-1", "Child owner: thread-231"] {
        assert_allowed(&format!(
            "{}\n{owner_metadata}\nChild branch codexy/231-branch-classification-guard was created after classification.\n{}",
            complete_child_classification(),
            ownership_footer()
        ))?;
    }
    Ok(())
}

#[test]
fn validator_allows_child_owned_owner_decision_with_parent_owned_negation() -> TestResult {
    assert_allowed(&format!(
        "{}\nChild branch codexy/231-branch-classification-guard was created after classification.\n{}",
        complete_child_classification_with_parent_owned_negation(),
        ownership_footer()
    ))
}

fn complete_parent_owned_owner_decision_classification() -> &'static str {
    r#"Ownership metadata source: current-thread-classified
Lane ownership: parent-owned
Task classification:
Lane type: validation
Secondary surfaces: workflow, validators
Owner decision: parent-owned for branch/worktree setup; parent owns implementation
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None"#
}

fn complete_child_classification() -> &'static str {
    r#"Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: child-owned implementation lane
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None"#
}

fn complete_child_classification_with_parent_owned_negation() -> &'static str {
    r#"Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: child-owned implementation lane (not parent-owned)
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None"#
}

fn ownership_footer() -> &'static str {
    "Review response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n"
}
