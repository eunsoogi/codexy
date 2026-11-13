use super::{TestResult, assert_profile_result};

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
            "an indented closer cannot hide later active security evidence",
            "Workflow profile: light\nContext: `carried\n    code`\nTask kind: security review",
        ),
        (
            "removing the genuine closer exposes security metadata before the fence",
            "Workflow profile: light\nContext: `carried\nTask kind: security review\ncode\n```text\nignored\n```",
        ),
        (
            "a blank line ends carried inline code before active security metadata",
            "Workflow profile: light\nContext: `open\n\nTask kind: security review\nclose`",
        ),
        (
            "an atx heading ends carried inline code before active security metadata",
            "Workflow profile: light\nContext: `open\n# New section\nTask kind: security review\nclose`",
        ),
        (
            "a whitespace-only indented blank line ends carried inline code before active security metadata",
            "Workflow profile: light\nContext: `open\n    \nTask kind: security review\nclose`",
        ),
    ] {
        assert_profile_result(name, evidence, false)?;
    }
    for marker in ["1.", "-", "+", "*"] {
        assert_profile_result(
            "a valid list-prefixed ATX heading ends carried inline code",
            &format!(
                "Workflow profile: light\nContext: `open\n{marker} # New section\nTask kind: security review\nclose`"
            ),
            false,
        )?;
    }
    for marker in ["01.", "000000001."] {
        assert_profile_result(
            "a leading-zero numeral-one ATX heading ends carried inline code",
            &format!(
                "Workflow profile: light\nContext: `open\n{marker} # New section\nTask kind: security review\nclose`"
            ),
            false,
        )?;
    }
    assert_profile_result(
        "a genuine closer keeps security metadata inside multiline inline code",
        "Workflow profile: light\nContext: `carried\nTask kind: security review\ncode`\n```text\nignored\n```",
        true,
    )?;
    for (name, evidence) in [
        (
            "an indented pseudo-heading does not end carried inline code",
            "Workflow profile: light\nContext: `carried\n    # Not a heading\nTask kind: security review\ncode`",
        ),
        (
            "seven markers do not form an atx heading",
            "Workflow profile: light\nContext: `carried\n####### Not a heading\nTask kind: security review\ncode`",
        ),
        (
            "a marker without following whitespace does not form an atx heading",
            "Workflow profile: light\nContext: `carried\n#Not a heading\nTask kind: security review\ncode`",
        ),
        (
            "a two-prefixed atx heading does not interrupt a carried paragraph",
            "Workflow profile: light\nContext: `carried\n2. # Not an interrupting heading\nTask kind: security review\ncode`",
        ),
        (
            "a ten-digit ordered marker does not interrupt a carried paragraph",
            "Workflow profile: light\nContext: `carried\n0000000001. # Not a list heading\nTask kind: security review\ncode`",
        ),
    ] {
        assert_profile_result(name, evidence, true)?;
    }
    Ok(())
}
