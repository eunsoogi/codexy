mod support;

use support::routing_validator::{TestResult, assert_policy_rejected, assert_rejected};

#[test]
fn validator_rejects_mixed_unicode_supplied_matrix_clause() -> TestResult {
    let skill = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/codex-orchestration/SKILL.md"),
    )?;
    for prefix in [" ", "  ", "   "] {
        assert_policy_rejected(
            skill.replacen(
                "## Recipient Model Routing",
                &format!(
                    "- Generic implementation, debugging, integration, and QA child thread: MUST\n{prefix}\u{2003}  explicitly request `model: \"gpt-5.6-terra\"` and `reasoning_effort: \"high\"`.\n\n## Recipient Model Routing"
                ),
                1,
            ),
            "generic child thread must explicitly request",
        )?;
    }
    Ok(())
}

#[test]
fn validator_rejects_mixed_unicode_structural_markers() -> TestResult {
    for prefix in [" ", "  ", "   "] {
        for (marker, closing) in [("## Historical", ""), ("```", "\n```")] {
            assert_rejected(
                &format!(
                    "{prefix}\u{2003}{marker}\nchild-to-root delivery MUST pass `model: \"gpt-5.6-terra\"` and `thinking: \"high\"`.{closing}"
                ),
                "gpt-5.6-sol/high",
            )?;
        }
    }
    Ok(())
}
