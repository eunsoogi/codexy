use super::workflow_profile_contract::{assert_profile_result, formal_classification};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn inactive_markdown_cannot_change_current_workflow_evidence() -> TestResult {
    for (name, evidence, expected) in [
        (
            "an indented formal table cannot satisfy strict proof",
            format!("Workflow profile: strict\n    {}", formal_classification()),
            false,
        ),
        (
            "a commented formal table cannot satisfy strict proof",
            format!("Workflow profile: strict\n<!--\n{}\n-->", formal_classification()),
            false,
        ),
        (
            "an indented historical boundary cannot erase active security work",
            "Task kind: security review\n    Review response: historical\nWorkflow profile: light".to_owned(),
            false,
        ),
        (
            "an indented historical profile cannot conflict with active light work",
            "Workflow profile: light\n    Workflow profile: strict".to_owned(),
            true,
        ),
        (
            "an inline-opened comment cannot satisfy strict proof",
            format!("Workflow profile: strict\nContext <!--\n{}\n-->", formal_classification()),
            false,
        ),
        (
            "an inline-opened comment cannot erase active security evidence",
            "Task kind: security review\nContext <!--\nReview response: historical\nWorkflow profile: light\n-->".to_owned(),
            false,
        ),
        (
            "text after an inline close remains active",
            format!("<!-- historical --> Workflow profile: strict\n{}", formal_classification()),
            true,
        ),
        (
            "active text before a same-line comment remains active",
            format!("Workflow profile: strict <!-- historical -->\n{}", formal_classification()),
            true,
        ),
        (
            "near-comment punctuation remains ordinary active text",
            format!("Workflow profile: strict\nContext < !--\n{}", formal_classification()),
            true,
        ),
    ] {
        assert_profile_result(name, &evidence, expected)?;
    }
    Ok(())
}

#[test]
fn only_valid_markdown_boundaries_can_hide_active_security_evidence() -> TestResult {
    for (name, evidence) in [
        (
            "a backtick in fence info does not open a fence",
            "```rust`\nTask kind: security review\nWorkflow profile: light",
        ),
        (
            "an inline-code comment marker does not open a comment",
            "Context: `<!--`\nTask kind: security review\nWorkflow profile: light",
        ),
    ] {
        assert_profile_result(name, evidence, false)?;
    }
    for (name, evidence) in [
        (
            "a valid fence hides security evidence",
            "```text\nTask kind: security review\n```\nWorkflow profile: light",
        ),
        (
            "a valid comment hides security evidence",
            "<!-- Task kind: security review -->\nWorkflow profile: light",
        ),
    ] {
        assert_profile_result(name, evidence, true)?;
    }
    Ok(())
}

#[test]
fn code_and_fence_boundaries_preserve_active_security_evidence() -> TestResult {
    for (name, evidence) in [
        (
            "an indented fence marker cannot close an active fence",
            "Task kind: security review\n```text\n    ```\nReview response: historical\n```\nWorkflow profile: light",
        ),
        (
            "a multiline code span cannot open an html comment",
            "Context: `<!--\nstill code`\nTask kind: security review\nWorkflow profile: light",
        ),
        (
            "ordinary multiline code leaves later metadata active",
            "Context: `ordinary\nmultiline code`\nTask kind: security review\nWorkflow profile: light",
        ),
    ] {
        assert_profile_result(name, evidence, false)?;
    }
    assert_profile_result(
        "an unindented fence marker closes an active fence",
        "Task kind: security review\n```text\n```\nReview response: historical\nWorkflow profile: light",
        true,
    )?;
    Ok(())
}

#[test]
fn prospective_inline_code_closers_stay_fence_local() -> TestResult {
    for (name, evidence) in [
        (
            "a backtick inside a later fence cannot close carried inline code",
            "Context: `carried\n```text\ncode`\n```\nTask kind: security review\nWorkflow profile: light",
        ),
        (
            "a fence without a matching backtick leaves later security evidence active",
            "Context: `carried\n```text\ncode\n```\nTask kind: security review\nWorkflow profile: light",
        ),
        (
            "a matching closer before a later fence remains ordinary inline code",
            "Context: `carried\ncode`\n```text\nignored\n```\nTask kind: security review\nWorkflow profile: light",
        ),
        (
            "removing the genuine closer exposes security metadata before the fence",
            "Workflow profile: light\nContext: `carried\nTask kind: security review\ncode\n```text\nignored\n```",
        ),
    ] {
        assert_profile_result(name, evidence, false)?;
    }
    assert_profile_result(
        "a genuine closer keeps security metadata inside multiline inline code",
        "Workflow profile: light\nContext: `carried\nTask kind: security review\ncode`\n```text\nignored\n```",
        true,
    )?;
    Ok(())
}

#[test]
fn comment_markers_precede_unclosed_code_spans() -> TestResult {
    for (name, evidence, expected) in [
        (
            "a comment ignores backticks before its closer",
            "<!-- ` --> Task kind: security review\nWorkflow profile: light", false,
        ),
        (
            "an unclosed backtick cannot hide later security metadata",
            "Context: `unclosed\nTask kind: security review\nWorkflow profile: light", false,
        ),
        (
            "a closed code span cannot block a later comment opener",
            "`prior` <!--\nTask kind: security review\n-->\nWorkflow profile: light", true,
        ),
    ] {
        assert_profile_result(name, evidence, expected)?;
    }
    Ok(())
}

#[test]
fn inert_lines_cannot_mutate_html_comment_state() -> TestResult {
    let indented_closer = format!(
            "Workflow profile: strict\nContext <!--\n    -->\n{}\n-->",
            formal_classification()
        );
    for (name, evidence) in [
        ("indented opener", "    <!-- inert opener\nTask kind: security review\nWorkflow profile: light"),
        ("one space then tab opener", " \t<!-- inert opener\nTask kind: security review\nWorkflow profile: light"),
        ("two spaces then tab opener", "  \t<!-- inert opener\nTask kind: security review\nWorkflow profile: light"),
        ("three spaces then tab opener", "   \t<!-- inert opener\nTask kind: security review\nWorkflow profile: light"),
        ("tab opener", "\t<!-- inert opener\nTask kind: security review\nWorkflow profile: light"),
        ("indented closer", indented_closer.as_str()),
        ("indented same-line delimiters", "    <!-- inert opener -->\nTask kind: security review\nWorkflow profile: light"),
        ("fenced delimiters", "```text\n<!-- inert fence opener -->\n```\nTask kind: security review\nWorkflow profile: light"),
    ] {
        assert_profile_result(
            name,
            evidence,
            false,
        )?;
    }
    assert_profile_result(
        "three spaces without a tab remain active markdown",
        "   <!-- active comment\nTask kind: security review\nWorkflow profile: light",
        true,
    )?;
    Ok(())
}
