use super::{TestResult, assert_profile_result};

#[test]
fn ordinary_list_boundaries_preserve_only_active_strict_signals() -> TestResult {
    for (name, evidence, expected) in [
        (
            "plus metadata stays active",
            "Workflow profile: light\n+ Task kind: security review",
            false,
        ),
        (
            "nested numeral-one metadata stays active",
            "Workflow profile: light\n1. 1. Task kind: security review",
            false,
        ),
        (
            "bullet ends carried inline code",
            "Workflow profile: light\nContext: `open\n- Task kind: security review\nclose`",
            false,
        ),
        (
            "ordered item ends carried inline code",
            "Workflow profile: light\nContext: `open\n1. Task kind: security review\nclose`",
            false,
        ),
        (
            "blank-separated numeral-two fenced metadata stays inactive",
            "Workflow profile: light\n\n2. ```text\n   Task kind: security review\n   ```",
            true,
        ),
        (
            "document-start numeral-two fenced metadata stays inactive",
            "2. ```text\n   Task kind: security review\n```\nWorkflow profile: light",
            true,
        ),
        (
            "document-start numeral-two plus metadata stays active",
            "2. + Task kind: security review",
            false,
        ),
        (
            "document-start numeral-two nested-one metadata stays active",
            "2. 1. Task kind: security review",
            false,
        ),
        (
            "ATX predecessor numeral-two plus metadata stays active",
            "# Section\n2. + Task kind: security review\nWorkflow profile: light",
            false,
        ),
        (
            "ATX predecessor numeral-two nested-one metadata stays active",
            "# Section\n2. 1. Task kind: security review\nWorkflow profile: light",
            false,
        ),
        (
            "ATX predecessor numeral-two fenced metadata stays inactive",
            "# Section\n2. ```text\n   Task kind: security review\n```\nWorkflow profile: light",
            true,
        ),
        (
            "ordered ATX predecessor plus metadata stays active",
            "2. # Section\n2. + Task kind: security review\nWorkflow profile: light",
            false,
        ),
        (
            "ordered ATX predecessor nested-one metadata stays active",
            "2. # Section\n2. 1. Task kind: security review\nWorkflow profile: light",
            false,
        ),
        (
            "ordered ATX predecessor fenced metadata stays inactive",
            "2. # Section\n2. ```text\n   Task kind: security review\n```\nWorkflow profile: light",
            true,
        ),
        (
            "inline-code-only predecessor plus metadata stays inactive",
            "`label`\n2. + Task kind: security review\nWorkflow profile: light",
            true,
        ),
        (
            "inline-code-only predecessor nested-one metadata stays inactive",
            "`label`\n2. 1. Task kind: security review\nWorkflow profile: light",
            true,
        ),
        (
            "inline-code-only predecessor fenced metadata stays active",
            "`label`\n2. ```text\n   Task kind: security review\n```\nWorkflow profile: light",
            false,
        ),
        (
            "indented-code predecessor plus metadata stays inactive",
            "Context\n    # Section\n2. + Task kind: security review\nWorkflow profile: light",
            true,
        ),
        (
            "indented-code predecessor nested-one metadata stays inactive",
            "Context\n    # Section\n2. 1. Task kind: security review\nWorkflow profile: light",
            true,
        ),
        (
            "indented-code predecessor fenced metadata stays active",
            "Context\n    # Section\n2. ```text\n   Task kind: security review\n```\nWorkflow profile: light",
            false,
        ),
        (
            "inline-code pseudo-heading plus metadata stays inactive",
            "`label` # Section\n2. + Task kind: security review\nWorkflow profile: light",
            true,
        ),
        (
            "inline-code pseudo-heading nested-one metadata stays inactive",
            "`label` # Section\n2. 1. Task kind: security review\nWorkflow profile: light",
            true,
        ),
        (
            "inline-code pseudo-heading fenced metadata stays active",
            "`label` # Section\n2. ```text\n   Task kind: security review\n```\nWorkflow profile: light",
            false,
        ),
        (
            "comment pseudo-heading plus metadata stays inactive",
            "<!-- note --> # Section\n2. + Task kind: security review\nWorkflow profile: light",
            true,
        ),
        (
            "comment pseudo-heading nested-one metadata stays inactive",
            "<!-- note --> # Section\n2. 1. Task kind: security review\nWorkflow profile: light",
            true,
        ),
        (
            "comment pseudo-heading fenced metadata stays active",
            "<!-- note --> # Section\n2. ```text\n   Task kind: security review\n```\nWorkflow profile: light",
            false,
        ),
    ] {
        assert_profile_result(name, evidence, expected)?;
    }
    Ok(())
}
