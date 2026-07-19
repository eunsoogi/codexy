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
fn canonical_table_activates_parent_authored_fix_guard() -> TestResult {
    assert_rejected(&format!(
        "{TABLE}\nReview response: parent-authored implementation commit abc123 fixed feedback\nMaintainer reassignment: none\n"
    ))
}

#[test]
fn canonical_table_activates_goal_reporting_guard() -> TestResult {
    assert_rejected(&format!(
        "{TABLE}\nSource thread id: parent-461\nGoal tool call: create_goal\n"
    ))
}

#[test]
fn numbered_lane_boundary_requires_a_fresh_table() -> TestResult {
    assert_rejected(&format!(
        "{TABLE}\nChild branch codexy/461-first was created after classification.\n1. Lane ownership: child-owned\nChild branch codexy/461-second was created without a fresh classification.\n"
    ))
}

#[test]
fn contradictory_or_multiple_owners_are_rejected() -> TestResult {
    for owner in [
        "current-thread-owned implementation lane; parent-owned coordination",
        "current-thread-owned implementation lane; not parent-owned and not implementation owner",
        "current-thread-owned 구현 lane; 부모 소유자가 아니며 구현 소유자도 아님",
    ] {
        assert_rejected(&setup_after(&TABLE.replace(
            "current-thread-owned implementation lane for #461",
            owner,
        )))?;
    }
    Ok(())
}

#[test]
fn raw_html_block_table_is_rejected() -> TestResult {
    for classification in [
        format!("<pre>\n{TABLE}\n</pre>"),
        format!("<div>\n{TABLE}\n</div>"),
        format!("<?instruction\n{TABLE}\n?>"),
    ] {
        assert_rejected(&setup_after(&classification))?;
    }
    Ok(())
}
