type TestResult = Result<(), Box<dyn std::error::Error>>;

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
