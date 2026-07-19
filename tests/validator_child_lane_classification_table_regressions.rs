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

#[test]
fn standalone_later_header_does_not_duplicate_a_gfm_table() -> TestResult {
    let evidence = format!(
        "{TABLE}\nChild branch codexy/461-table was created after classification.\n| Task classification | Decision |\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n"
    );
    let output = run_validator(&evidence)?;
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
