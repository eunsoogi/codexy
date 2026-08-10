type TestResult = Result<(), Box<dyn std::error::Error>>;

use crate::support::wiki_core_contract::{validate_core_skill, validate_migration_rules};

const REMOVED_WORKFLOWS: &[&str] = &[
    "collect", "plan", "project", "inventory", "dataset", "archive", "ll", "status", "session",
    "session-capture", "rehydrate", "feedback", "feedback-capture",
];

#[test]
fn parsed_skill_shape_rejects_each_core_identity_mutation() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let skill = std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/SKILL.md"))?;
    for (name, mutation) in [
        ("heading", skill.replacen("## Core workflow", "## Retired workflow", 1)),
        ("inventory", skill.replacen("init → ingest → compile → query → refresh", "init → query", 1)),
        ("link", skill.replacen("[Migration](references/migration.md)", "Migration", 1)),
        ("removed", format!("{skill}\n`collect`")),
    ] {
        assert!(validate_core_skill(&mutation, REMOVED_WORKFLOWS).is_err(), "{name}");
    }
    Ok(())
}

#[test]
fn normalized_migration_rules_reject_each_required_rule_mutation() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let guide = std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/references/migration.md"))?;
    for (name, required) in [
        ("preservation", "MUST preserve existing `raw/`, `wiki/`, `_index.md`, and `log.md`"),
        ("no deletion", "MUST NOT delete, overwrite, or rename existing topic data"),
        ("source scalar", "`sources:` scalar"),
        ("provenance gap", "provenance gap"),
        ("preflight", "MUST validate every referenced provenance and freshness input before any log"),
        ("write", "MUST append one migration entry"),
    ] {
        let mutation = guide.replacen(required, "retired rule", 1);
        assert!(validate_migration_rules(&mutation).is_err(), "{name}");
    }
    Ok(())
}
