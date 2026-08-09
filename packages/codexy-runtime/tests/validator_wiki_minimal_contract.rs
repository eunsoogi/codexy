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
    let mutations = vec![
        ("missing retract", original.replacen("| `retract` | Merge |", "", 1)),
        ("missing session capture", original.replacen("| `session-capture` | Remove |", "", 1)),
        ("duplicate workflow", original.replacen("| `compile` | Keep |", "| `compile` | Keep |\n| `compile` | Keep |", 1)),
        ("conflicting workflow", original.replacen("| `init` | Keep |", "| `init` | Merge |", 1)),
        ("wrong workflow heading", original.replacen("## Current workflow disposition", "### Current workflow disposition", 1)),
        ("wrong child heading", original.replacen("### Ingest", "#### Ingest", 1)),
        ("wrong essential parent", original.replacen("### Ingest", "## Detached\n\n### Ingest", 1)),
        ("wrong measurable parent", original.replacen("### Machine-checkable limits", "## Detached\n\n### Machine-checkable limits", 1)),
        ("malformed separator", original.replacen("| --- | --- | --- |", "| -- | --- | --- |", 1)),
        ("backtick fenced row", original.replacen("| `retract` | Merge |", "```text\n| `retract` | Merge |", 1)),
        ("tilde fenced row", original.replacen("| `retract` | Merge |", "~~~text\n| `retract` | Merge |", 1)),
        ("tilde wrapped assignments", original.replacen("```text\nquery.max_index_files", "~~~text\n```text\nquery.max_index_files", 1).replacen("```\n\nFor valid", "```\n~~~\n\nFor valid", 1)),
        ("missing aggregate", original.replacen("freshness.score =", "freshness.aggregate =", 1)),
        ("nonnumeric assignment", original.replacen("query.max_index_files = 3", "query.max_index_files = three", 1)),
        ("duplicate numeric assignment", original.replacen("query.max_index_files = 3", "query.max_index_files = 3\nquery.max_index_files = 3", 1)),
        ("outside assignment", original.replacen("```\n\nFor valid", "```\nfreshness.score = round_half_up(decay(source_age) + decay(verification_age) + decay(compilation_age) + source_chain)\n\nFor valid", 1)),
        ("conflicting numeric assignment", original.replacen("query.max_index_files = 3", "query.max_index_files = 4", 1)),
        ("missing decay", original.replacen(ASSIGNMENTS[9].1, "freshness.decay = unspecified", 1)),
        ("duplicate decay", original.replacen(ASSIGNMENTS[9].1, &format!("{}\n{}", ASSIGNMENTS[9].1, ASSIGNMENTS[9].1), 1)),
        ("future date rule", original.replacen("freshness.future_date = 0", "freshness.future_date = clamp(age_days, 0, infinity)", 1)),
        ("source age aggregate", original.replacen("freshness.source_age = average(age_days across resolvable sources)", "freshness.source_age = min(age_days across resolvable sources)", 1)),
    ];
    let accepted: Vec<_> = mutations.into_iter().filter_map(|(name, mutation)| {
        validate_contract(&mutation).is_ok().then_some(name)
    }).collect();
    assert!(accepted.is_empty(), "accepted invalid variants: {accepted:?}");
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
