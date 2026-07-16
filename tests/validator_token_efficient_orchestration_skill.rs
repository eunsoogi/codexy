#[path = "structured_contract.rs"]
mod structured_contract;
#[path = "structured_contract_rules/mod.rs"]
mod structured_contract_rules;

#[test]
fn token_efficient_orchestration_skill_preserves_proof_gates()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let token_skill = std::fs::read_to_string(
        root.join("plugins/codexy/skills/token-efficient-orchestration/SKILL.md"),
    )?;
    let orchestration =
        std::fs::read_to_string(root.join("plugins/codexy/skills/codex-orchestration/SKILL.md"))?;

    structured_contract::assert_rules(
        &structured_contract::Contract::markdown(&token_skill),
        structured_contract_rules::TOKEN_CONTAINMENT,
    );
    structured_contract::assert_rules(
        &structured_contract::Contract::markdown(&orchestration),
        structured_contract_rules::ORCHESTRATION,
    );
    Ok(())
}
