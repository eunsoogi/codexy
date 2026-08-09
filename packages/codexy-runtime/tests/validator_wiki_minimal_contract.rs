type TestResult = Result<(), Box<dyn std::error::Error>>;

use std::path::Path;

#[test]
fn wiki_skill_exposes_the_minimal_contract() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let skill = std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/SKILL.md"))?;
    let contract = std::fs::read_to_string(
        root.join("plugins/codexy/skills/wiki/references/minimal-contract.md"),
    )?;

    assert!(skill.contains("[Minimal Contract](references/minimal-contract.md)"));
    for heading in [
        "## Essential contract",
        "## Measurable criteria",
        "## Current workflow disposition",
    ] {
        assert!(contract.contains(heading), "missing {heading}");
    }
    for (workflow, disposition) in [
        ("init", "Keep"),
        ("ingest", "Keep"),
        ("ingest-collection", "Merge"),
        ("collect", "Remove"),
        ("compile", "Keep"),
        ("query", "Keep"),
        ("refresh", "Keep"),
        ("lint", "Keep"),
        ("librarian", "Merge"),
        ("audit", "Merge"),
        ("research", "Merge"),
        ("output", "Merge"),
        ("plan", "Remove"),
        ("project", "Remove"),
        ("inventory", "Remove"),
        ("dataset", "Remove"),
        ("archive", "Remove"),
        ("ll", "Remove"),
        ("assess", "Merge"),
    ] {
        assert!(
            contract.contains(&format!("| `{workflow}` | {disposition} |")),
            "missing {workflow} disposition"
        );
    }
    for criterion in ["Context efficiency", "Traceability", "Freshness"] {
        assert!(contract.contains(criterion), "missing {criterion}");
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

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
