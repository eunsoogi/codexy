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

fn assert_allowed(evidence: &str) -> TestResult {
    assert!(run_validator(evidence)?.status.success());
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
        "current-thread-owned implementation lane; not parent-owned but parent-owned coordination",
        "current-thread-owned implementation lane; no parent implementation edits but parent implementation owner",
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
        format!("<!--\n{TABLE}\n-->"),
        format!("<div>\n{TABLE}\n</div>"),
        format!("<?instruction\n{TABLE}\n?>"),
        format!("<Warning>\n{TABLE}\n</Warning>"),
        format!("<!DOCTYPE\nhtml\n{TABLE}\n>"),
        format!("<![CDATA[\n{TABLE}\n]]>"),
    ] {
        let output = run_validator(&setup_after(&classification))?;
        assert!(
            !output.status.success(),
            "raw HTML classification must be rejected: {classification}"
        );
    }
    Ok(())
}

#[test]
fn table_after_blank_terminated_html_block_is_allowed() -> TestResult {
    assert_allowed(&setup_after(&format!("<div>\nraw html\n\n{TABLE}")))
}

#[test]
fn table_after_any_type_one_closer_is_allowed() -> TestResult {
    assert_allowed(&setup_after(&format!(
        "<script>\nraw html\n</style>\n{TABLE}"
    )))
}

#[test]
fn indented_html_tag_does_not_start_a_raw_html_block() -> TestResult {
    for prefix in ["    ", "\t"] {
        assert_allowed(&setup_after(&format!("{prefix}<div>\n{TABLE}")))?;
    }
    Ok(())
}

#[test]
fn indented_html_comment_does_not_start_a_raw_html_block() -> TestResult {
    for prefix in ["    ", "\t"] {
        assert_allowed(&setup_after(&format!("{prefix}<!--\n\n{TABLE}")))?;
    }
    Ok(())
}

#[test]
fn lowercase_cdata_token_does_not_start_a_raw_html_block() -> TestResult {
    assert_allowed(&setup_after(&format!("<![cdata[\nraw text\n\n{TABLE}")))
}

#[test]
fn slash_after_type_one_name_does_not_start_that_html_block() -> TestResult {
    for tag in ["pre", "script", "style"] {
        assert_allowed(&setup_after(&format!("<{tag}/garbage>\n{TABLE}")))?;
    }
    Ok(())
}

#[test]
fn malformed_type_six_and_seven_tags_do_not_hide_the_table() -> TestResult {
    for malformed in ["<div/garbage>", "<Warning ???>"] {
        assert_allowed(&setup_after(&format!("{malformed}\n{TABLE}")))?;
    }
    Ok(())
}

#[test]
fn type_seven_tag_does_not_interrupt_a_paragraph() -> TestResult {
    for paragraph in [
        "paragraph",
        "1234567890. paragraph",
        "paragraph\n2. continuation",
        "paragraph\n1.",
        "paragraph\n+",
        "paragraph\n*",
        "paragraph\n    lazy continuation",
    ] {
        assert_allowed(&setup_after(&format!("{paragraph}\n<Warning>\n{TABLE}")))?;
    }
    Ok(())
}

#[test]
fn type_seven_tag_after_block_boundary_starts_html_block() -> TestResult {
    for boundary in [
        "> paragraph",
        ">paragraph",
        ">> paragraph",
        ">>paragraph",
        "1. paragraph",
        "01. paragraph", "000000001. paragraph",
        "01) paragraph", "000000001) paragraph",
        "+", "*", "1.", "000000001)",
        "***",
        "___",
        "#",
        "###",
    ] {
        assert_rejected(&setup_after(&format!("{boundary}\n<Warning>\n{TABLE}")))?;
    }
    Ok(())
}

#[test]
fn marker_terminated_html_blocks_allow_the_following_table() -> TestResult {
    for html in [
        "<!-- raw -->",
        "<?raw?>",
        "<!DOCTYPE html>",
        "<![CDATA[raw]]>",
    ] {
        assert_allowed(&setup_after(&format!("{html}\n{TABLE}")))?;
    }
    Ok(())
}

#[test]
fn malformed_extra_cell_duplicate_is_rejected() -> TestResult {
    assert_rejected(&setup_after(&format!(
        "{TABLE}\n| Atomic scope | duplicated | ignored |"
    )))
}

#[test]
fn escaped_pipe_inside_a_value_is_allowed() -> TestResult {
    assert_allowed(&setup_after(
        &TABLE.replace("workflow, validators", r"workflow \| validators"),
    ))
}

#[test]
fn hidden_fake_table_does_not_duplicate_the_rendered_table() -> TestResult {
    let indented = TABLE
        .lines()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    for hidden in [
        format!("```text\n{TABLE}\n```"),
        indented,
        format!("<div>\n{TABLE}\n\n"),
    ] {
        assert_allowed(&setup_after(&format!("{hidden}\n{TABLE}")))?;
    }
    Ok(())
}

#[test]
fn non_rendering_markers_do_not_leak_across_contexts() -> TestResult {
    for prefix in [
        "```text\n<div>\n```",
        "```text\n<!--\n```",
        "```text\n<?raw\n```",
        "<!--\n<div>\n-->",
        "<div>\n```\n",
    ] {
        assert_allowed(&setup_after(&format!("{prefix}\n{TABLE}")))?;
    }
    Ok(())
}

#[test]
fn mixed_space_tab_indented_table_is_rejected() -> TestResult {
    for spaces in [" ", "  ", "   "] {
        let classification = TABLE
            .lines()
            .map(|line| format!("{spaces}\t{line}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_rejected(&setup_after(&classification))?;
    }
    Ok(())
}
