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
fn validator_rejects_child_owner_setup_without_task_classification() -> TestResult {
    for owner_evidence in [
        "Child owner: codex thread 123",
        "- Lane ownership: child-owned",
    ] {
        assert_rejected(&format!(
            r#"{owner_evidence}
Child branch codexy/228-reject-generic-reviewer-gate-sentinel-proof was created immediately after thread rename.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
        ))?;
    }
    Ok(())
}

#[test]
fn validator_allows_absent_child_owner_metadata_without_child_lane_context() -> TestResult {
    assert_allowed(
        r#"Child owner: none assigned yet
Child branch codexy/231-branch-classification-guard was created immediately after thread rename.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_allows_absent_before_classification_clause_after_valid_setup() -> TestResult {
    assert_allowed(&format!(
        "Lane ownership: child-owned\n{}\nChild branch codexy/231-branch-classification-guard was created after classification; no child branch was created before task classification.\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n",
        canonical_table()
    ))
}

#[test]
fn validator_rejects_codexy_worktree_setup_before_classification() -> TestResult {
    assert_rejected(
        r#"Lane ownership: child-owned
Worktree for codexy/231-branch-classification-guard was created before task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_current_thread_owner_setup_before_classification() -> TestResult {
    assert_rejected(
        r#"Owner decision: current-thread-owned implementation lane for #231
Child branch codexy/231-branch-classification-guard was created before task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_git_switch_create_before_classification() -> TestResult {
    assert_rejected(
        r#"Lane ownership: child-owned
Child ran git switch -c codexy/231-branch-classification-guard before task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_unqualified_setup_before_classification() -> TestResult {
    for setup in [
        "Created branch before task classification.",
        "Created worktree before task classification.",
        "Worktree was created before task classification.",
    ] {
        assert_rejected(&format!(
            "Lane ownership: child-owned\n{setup}\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n"
        ))?;
    }
    Ok(())
}

#[test]
fn validator_allows_codexy_worktree_setup_after_classification() -> TestResult {
    assert_allowed(&format!(
        "Lane ownership: child-owned\n{}\nWorktree for codexy/231-branch-classification-guard was created after task classification.\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n",
        canonical_table()
    ))
}

#[test]
fn rendered_table_is_the_only_classification_source() -> TestResult {
    let table = r#"| Task classification | Decision |
| --- | --- |
| Lane type | implementation |
| Secondary surfaces | workflow, validators |
| Owner decision | current-thread-owned child implementation lane |
| Atomic scope | issue-sized |
| Required skills | task-classification, codex-orchestration, git-workflow |
| Required tools/evidence | goal, plan, codegraph, LSP, Sentinel |
| First allowed action | create branch after classification |
| Stop/blocker | None |
"#;
    let footer = "Review response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n";
    assert_allowed(&format!(
        "{table}\nChild branch codexy/461-table was created after classification.\n{footer}"
    ))?;
    assert_rejected(&format!(
        "{table}\nReview response: parent-authored implementation commit abc123 fixed feedback\n"
    ))?;
    assert_rejected(&format!("{table}\nSource thread id: parent-461\nGoal tool call: create_goal\n"))?;
    assert_rejected(&format!(
        "Lane ownership: child-owned\n```text\n{table}```\nChild branch codexy/461-table was created after classification.\n{footer}"
    ))?;
    assert_rejected(&format!(
        "{table}\nPR: #468\nReview response: parent-authored implementation commit abc123 fixed feedback\n"
    ))
}

#[test]
fn validator_rejects_incomplete_child_table_for_ownership_and_goal_evidence() -> TestResult {
    let incomplete = canonical_table().replacen("| Stop/blocker | None |\n", "", 1);
    assert_rejected(&format!(
        "{incomplete}\nReview response: parent-authored implementation commit abc123 fixed feedback\n"
    ))?;
    assert_rejected(&format!(
        "{incomplete}\nSource thread id: parent-461\nGoal tool call: create_goal\n"
    ))
}

#[test]
fn validator_keeps_table_ownership_across_handoff_metadata_before_pr() -> TestResult {
    assert_rejected(&format!(
        "{}\nIssue: #461\nBranch: eunsoogi/461-main-rendered-table\nWorktree path: /tmp/codexy-461\nPR: #468\nReview response: parent-authored implementation commit abc123 fixed feedback\n",
        canonical_table()
    ))
}

#[test]
fn validator_rejects_no_blank_table_handoff_for_setup_ownership_and_goal_evidence() -> TestResult {
    let handoff = format!(
        "{}Issue: #461\nBranch: eunsoogi/461-main-rendered-table\nWorktree path: /tmp/codexy-461\nPR: #468\n",
        canonical_table()
    );
    assert_rejected(&format!(
        "{handoff}Child branch codexy/461-table was created before task classification.\n"
    ))?;
    assert_rejected(&format!(
        "{handoff}Review response: parent-authored implementation commit abc123 fixed feedback\n"
    ))?;
    assert_rejected(&format!(
        "{handoff}Source thread id: parent-461\nGoal tool call: create_goal\n"
    ))
}

#[test]
fn validator_allows_korean_current_thread_implementation_owner_after_classification() -> TestResult {
    assert_allowed(&format!(
        "Lane ownership: child-owned\n{}\nChild branch codexy/461-table was created after classification.\n",
        canonical_table().replace(
            "current-thread-owned child implementation lane",
            "current-thread-owned — 현재 작업이 구현을 소유함",
        )
    ))
}

#[test]
fn task_classification_skill_requires_the_compact_table() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let skill = std::fs::read_to_string(root.join("plugins/codexy/skills/task-classification/SKILL.md"))?;
    let prompt: serde_yaml::Value = serde_yaml::from_str(&std::fs::read_to_string(
        root.join("plugins/codexy/skills/task-classification/agents/openai.yaml"),
    )?)?;
    assert_eq!(
        skill
            .lines()
            .filter(|line| *line == "| Task classification | Decision |")
            .count(),
        1
    );
    assert_eq!(
        prompt["interface"]["default_prompt"].as_str(),
        Some("You MUST use $task-classification first and emit one ordered eight-row GFM table naming lane type, secondary surfaces, owner decision, atomic scope, required skills, required tools/evidence, first allowed action, and blocker before Codexy setup, delegation, implementation, PR, review-response, or merge work begins.")
    );
    Ok(())
}

fn canonical_table() -> &'static str {
    "| Task classification | Decision |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | workflow, validators |\n| Owner decision | current-thread-owned child implementation lane |\n| Atomic scope | issue-sized |\n| Required skills | task-classification, codex-orchestration, git-workflow |\n| Required tools/evidence | goal, plan, codegraph, LSP, Sentinel |\n| First allowed action | create branch after classification |\n| Stop/blocker | None |\n"
}
