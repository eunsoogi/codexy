use crate::support::{TestResult, stderr, validator_instruction_policy};

use super::validator_sentinel_scope_policy_fixture::{
    NONTERMINAL_OBSERVATION_CLAUSE, fixture,
};

const SURFACES: &[&str] = &[
    "skills/codex-orchestration/SKILL.md",
    "skills/codex-orchestration/references/classification-and-control.md",
    "skills/proof-driven-completion/SKILL.md",
    "skills/token-efficient-orchestration/SKILL.md",
];

#[test]
fn validator_requires_nonterminal_observation_policy_on_every_surface() -> TestResult {
    let fixture = fixture()?;
    for relative in SURFACES {
        fixture.reset_file(std::path::Path::new(relative))?;
        let plugin_root = fixture.root();
        let path = plugin_root.join(relative);
        let text = std::fs::read_to_string(&path)?;
        std::fs::write(
            &path,
            text.replace(NONTERMINAL_OBSERVATION_CLAUSE, "removed clause."),
        )?;

        let output = validator_instruction_policy(&plugin_root)?;
        assert!(
            !output.status.success(),
            "{relative} unexpectedly passed without the non-terminal policy"
        );
        assert!(
            stderr(&output).contains("non-terminal observation clause"),
            "{relative}: {}",
            stderr(&output)
        );
    }
    Ok(())
}
