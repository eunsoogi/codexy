type TestResult = Result<(), Box<dyn std::error::Error>>;

use crate::support::{
    wiki_core_contract::validate_core_skill,
    wiki_migration_rules::validate_migration_rules,
};

const REMOVED_WORKFLOWS: &[&str] = &[
    "collect", "plan", "project", "inventory", "dataset", "archive", "ll", "status", "session",
    "session-capture", "rehydrate", "feedback", "feedback-capture",
];

#[test]
fn core_skill_rejects_an_inline_topic_root_operand_in_an_implicit_action() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let skill = std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/SKILL.md"))?;
    let mutation = skill.replacen(
        "The caller MUST NOT search, select, or\ninitialize a topic root implicitly.",
        "The caller MUST NOT search, select, or\ninitialize a topic root implicitly. The caller MUST search for a `topic root` implicitly.",
        1,
    );
    assert!(validate_core_skill(&mutation, REMOVED_WORKFLOWS).is_err());
    Ok(())
}

#[test]
fn migration_rules_keep_plain_log_identity_exact() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let guide = std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/references/migration.md"))?;
    let plain_final = guide.replacen("to `log.md` as the final", "to log.md as the final", 1);
    let plain_result = validate_migration_rules(&plain_final);
    assert!(plain_result.is_ok(), "plain exact log.md: {plain_result:?}");
    for decoy in ["log md", "log-md", "log/md", "log.md.bak"] {
        let mutation = guide.replacen(
            "3. MUST stage all derived changes",
            &format!("3. Another system MUST append a backup to {decoy} before staging. MUST stage all derived changes"),
            1,
        );
        assert!(validate_migration_rules(&mutation).is_ok(), "decoy: {decoy}");
    }
    Ok(())
}
