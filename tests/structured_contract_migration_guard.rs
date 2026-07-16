#[path = "structured_contract_guard/mod.rs"]
mod structured_contract_guard;

use structured_contract_guard::{comparison_counts, repository_violations, scan_source};

#[test]
fn new_contract_tests_cannot_add_unstructured_substring_assertions() {
    let violations = repository_violations().expect("migration guard must inspect test changes");
    assert!(
        violations.is_empty(),
        "new governed substring assertions need structured rules: {violations:?}"
    );
}

#[test]
fn governed_migration_reduces_direct_substring_assertions() {
    let paths = [
        "tests/token_quota_containment.rs",
        "tests/validator_runtime_heartbeat_contract.rs",
        "tests/validator_subagent_delegation.rs",
        "tests/validator_token_efficient_orchestration_skill.rs",
    ];
    let (before, after) = comparison_counts(&paths).expect("comparison must inspect origin/main");
    eprintln!("governed direct substring assertions: {before} -> {after}");
    assert!(
        after < before,
        "migration did not reduce {before} -> {after}"
    );
}

#[test]
fn guard_rejects_multiline_and_assert_eq_governed_substring_checks() {
    for assertion in [
        "assert!(\n    skill.contains(\"required policy\")\n);",
        "assert_eq!(skill.contains(\"required policy\"), true);",
        "assert!(skill.contains (\"required policy\"));",
    ] {
        let source = format!(
            "let skill = std::fs::read_to_string(root.join(\"plugins/codexy/skills/demo/SKILL.md\"))?;\n{assertion}"
        );
        assert_eq!(scan_source(&source).len(), 1, "{assertion}");
    }
}

#[test]
fn guard_allows_diagnostics_and_requires_a_substantive_rationale() {
    let diagnostic = "assert!(stderr.contains(\"validator failed\"));";
    assert!(scan_source(diagnostic).is_empty());

    let unknown = "assert!(snapshot.contains(\"heading\"));";
    assert_eq!(scan_source(unknown).len(), 1);
    let blank = concat!(
        "// structured-contract: non-contract substring rationale:\n",
        "assert!(snapshot.contains(\"heading\"));"
    );
    assert_eq!(scan_source(blank).len(), 1);
    let explained = concat!(
        "// structured-contract: non-contract substring rationale: verifies rendered CLI output\n",
        "assert!(snapshot.contains(\"heading\"));"
    );
    assert!(scan_source(explained).is_empty());

    let governed = concat!(
        "let skill = std::fs::read_to_string(\"plugins/codexy/skills/demo/SKILL.md\")?;\n",
        "// structured-contract: non-contract substring rationale: verifies rendered CLI output\n",
        "assert!(skill.contains(\"heading\"));"
    );
    assert_eq!(scan_source(governed).len(), 1);

    let indirect = concat!(
        "let path = root.join(\"plugins/codexy/skills/demo/SKILL.md\");\n",
        "let skill = std::fs::read_to_string(path)?;\n",
        "// structured-contract: non-contract substring rationale: verifies rendered CLI output\n",
        "assert!(skill.contains(\"MUST retain\"));"
    );
    assert_eq!(scan_source(indirect).len(), 1);
}

#[test]
fn guard_ignores_assertion_text_inside_raw_strings_and_block_comments() {
    let source = concat!(
        "let sample = r##\"assert!(snapshot.contains(\\\"heading\\\"));\"##;\n",
        "/* assert!(snapshot.contains(\"heading\")); */\n"
    );
    assert!(scan_source(source).is_empty());
}

#[test]
fn guard_handles_character_literal_parentheses_inside_assertions() {
    let source = concat!(
        "let skill = std::fs::read_to_string(\"plugins/codexy/skills/demo/SKILL.md\")?;\n",
        "assert!(skill.contains('('));"
    );
    assert_eq!(scan_source(source).len(), 1);
}
