use std::process::Output;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TABLE: &str = r#"| Task classification | Decision |
| --- | --- |
| Lane type | implementation |
| Secondary surfaces | workflow, validators |
| Owner decision | current-thread-owned implementation lane for #461 |
| Atomic scope | issue-sized |
| Required skills | task-classification, test-driven-development |
| Required tools/evidence | goal, plan, codegraph, LSP, Sentinel |
| First allowed action | create branch after classification |
| Stop/blocker | None |"#;

fn run_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;
    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

fn assert_rejected(evidence: &str) -> TestResult {
    assert!(!run_validator(evidence)?.status.success());
    Ok(())
}

fn setup_after(classification: &str) -> String {
    format!(
        "{classification}\nChild branch codexy/461-table was created after classification.\n"
    )
}

#[test]
fn mismatched_type_one_closer_keeps_the_table_hidden() -> TestResult {
    assert_rejected(&setup_after(&format!(
        "<script>\nraw html\n</style>\n{TABLE}"
    )))
}

#[test]
fn textarea_type_one_block_keeps_the_table_hidden_after_a_blank_line() -> TestResult {
    assert_rejected(&setup_after(&format!("<textarea>\n\n{TABLE}")))
}
