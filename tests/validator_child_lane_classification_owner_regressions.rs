use std::process::Output;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TABLE: &str = r#"| Task classification | Decision |
| --- | --- |
| Lane type | implementation |
| Secondary surfaces | workflow, validators |
| Owner decision | current-thread-owned implementation lane for #461 |
| Atomic scope | issue-sized |
| Required skills | task-classification, codex-orchestration, git-workflow |
| Required tools/evidence | goal, plan, codegraph, LSP, Sentinel |
| First allowed action | create branch after classification |
| Stop/blocker | None |"#;

fn run_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;
    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

fn assert_allowed(owner: &str) -> TestResult {
    let evidence = format!(
        "{}\nChild branch codexy/461-table was created after classification.\n",
        TABLE.replace("current-thread-owned implementation lane for #461", owner)
    );
    assert!(run_validator(&evidence)?.status.success(), "{owner}");
    Ok(())
}

#[test]
fn localized_and_unrelated_denials_preserve_current_thread_ownership() -> TestResult {
    for owner in [
        "current-thread-owned — ce fil possède l’implémentation",
        "current-thread-owned implementation lane; no ownership conflict",
        "current-thread-owned implementation lane; without modifying parent files",
    ] {
        assert_allowed(owner)?;
    }
    Ok(())
}
