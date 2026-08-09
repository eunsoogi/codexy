type TestResult = Result<(), Box<dyn std::error::Error>>;

use std::path::Path;

use crate::support::wiki_minimal_contract::{ASSIGNMENTS, validate_contract};

#[test]
fn wiki_skill_exposes_a_complete_measurable_minimal_contract() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let skill = std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/SKILL.md"))?;
    let contract = contract(&root)?;
    assert!(skill.contains("[Minimal Contract](references/minimal-contract.md)"));
    validate_contract(&contract)?;
    Ok(())
}

#[test]
fn contract_parser_rejects_each_structural_contract_violation() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let original = contract(&root)?;
    let mutations = [
        original.replacen("| `retract` | Merge |", "", 1),
        original.replacen("| `session-capture` | Remove |", "", 1),
        original.replacen("| `compile` | Keep |", "| `compile` | Keep |\n| `compile` | Keep |", 1),
        original.replacen("| `init` | Keep |", "| `init` | Merge |", 1),
        original.replacen("## Current workflow disposition", "### Current workflow disposition", 1),
        original.replacen("### Ingest", "#### Ingest", 1),
        original.replacen("| --- | --- | --- |", "| -- | --- | --- |", 1),
        original.replacen("| `retract` | Merge |", "```text\n| `retract` | Merge |", 1),
        original.replacen("freshness.score =", "freshness.aggregate =", 1),
        original.replacen("query.max_index_files = 3", "query.max_index_files = three", 1),
        original.replacen("query.max_index_files = 3", "query.max_index_files = 3\nquery.max_index_files = 3", 1),
        original.replacen("```\n\nFor valid", "```\nfreshness.score = round_half_up(decay(source_age) + decay(verification_age) + decay(compilation_age) + source_chain)\n\nFor valid", 1),
        original.replacen("query.max_index_files = 3", "query.max_index_files = 4", 1),
        original.replacen(ASSIGNMENTS[9].1, "freshness.decay = unspecified", 1),
        original.replacen(ASSIGNMENTS[9].1, &format!("{}\n{}", ASSIGNMENTS[9].1, ASSIGNMENTS[9].1), 1),
        original.replacen("freshness.future_date = 0", "freshness.future_date = clamp(age_days, 0, infinity)", 1),
        original.replacen("freshness.source_age = max(age_days across resolvable sources)", "freshness.source_age = min(age_days across resolvable sources)", 1),
    ];
    for mutation in mutations {
        assert!(validate_contract(&mutation).is_err());
    }
    Ok(())
}

#[test]
fn minimal_contract_uses_canonical_instruction_policy_forms() -> TestResult {
    let fixture = crate::support::instruction_policy_fixture(Path::new(
        "skills/wiki/references/minimal-contract.md",
    ))?;
    let contract_path = fixture.path();
    let contract = std::fs::read_to_string(&contract_path)?;

    let output = crate::support::validator_instruction_policy_file(&contract_path)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));

    let invalid_prohibition = contract.replace(
        "MUST NOT overwrite raw history.",
        "never overwrites raw history.",
    );
    std::fs::write(&contract_path, invalid_prohibition)?;
    let output = crate::support::validator_instruction_policy_file(&contract_path)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("prohibitions must use MUST NOT"));

    let invalid_imperative = contract.replace(
        "MUST report why it remains current.",
        "report why it remains current.",
    );
    std::fs::write(&contract_path, invalid_imperative)?;
    let output = crate::support::validator_instruction_policy_file(&contract_path)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    Ok(())
}

fn contract(root: &Path) -> Result<String, std::io::Error> {
    std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/references/minimal-contract.md"))
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
