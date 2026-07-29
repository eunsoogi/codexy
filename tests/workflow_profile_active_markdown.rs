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
fn inert_lines_cannot_mutate_html_comment_state() -> TestResult {
    let indented_closer = format!(
            "Workflow profile: strict\nContext <!--\n    -->\n{}\n-->",
            formal_classification()
        );
    for (name, evidence) in [
        ("indented opener", "    <!-- inert opener\nTask kind: security review\nWorkflow profile: light"),
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
    Ok(())
}
