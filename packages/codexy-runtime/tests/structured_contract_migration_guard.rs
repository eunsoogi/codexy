#[path = "structured_contract_guard/mod.rs"]
mod structured_contract_guard;
#[path = "structured_contract_guard/repository_tests.rs"]
mod repository_tests;

use structured_contract_guard::{comparison_counts, repository_violations, scan_source};

const GOVERNED_RUNTIME_TESTS: &[&str] = &[
    "tests/token_quota_containment.rs",
    "tests/validator_runtime_heartbeat_contract.rs",
    "tests/validator_subagent_delegation.rs",
    "tests/validator_token_efficient_orchestration_skill.rs",
];

fn migration_counts_preserve_guard(before: usize, after: usize) -> bool {
    if before == 0 {
        after == 0
    } else {
        after < before
    }
}

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
    let (before, after) =
        comparison_counts(GOVERNED_RUNTIME_TESTS).expect("comparison must inspect origin/main");
    eprintln!("governed direct substring assertions: {before} -> {after}");
    assert!(
        migration_counts_preserve_guard(before, after),
        "migration must reduce a positive baseline and preserve a zero baseline: {before} -> {after}"
    );
}

#[test]
fn migration_paths_use_runtime_local_reads_and_repository_baselines() {
    let repository = codexy_runtime::paths::repository_root();
    let runtime = codexy_runtime::paths::runtime_package_root();
    for path in GOVERNED_RUNTIME_TESTS {
        assert!(runtime.join(path).is_file(), "missing current runtime path {path}");
        assert!(!repository.join(path).is_file(), "stale root path exists {path}");
    }
    assert!(comparison_counts(GOVERNED_RUNTIME_TESTS).is_ok());
    assert!(comparison_counts(&["tests/missing-governed-path.rs"]).is_err());
}

#[test]
fn migration_control_requires_reduction_before_steady_state() {
    assert!(migration_counts_preserve_guard(33, 0));
    assert!(!migration_counts_preserve_guard(33, 33));
    assert!(!migration_counts_preserve_guard(33, 34));
    assert!(migration_counts_preserve_guard(0, 0));
    assert!(!migration_counts_preserve_guard(0, 1));
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
fn guard_rejects_debug_assertion_macro_substring_checks() {
    for assertion in [
        "debug_assert!(skill.contains(\"required policy\"));",
        "debug_assert_eq!(skill.contains(\"required policy\"), true);",
        "debug_assert_ne!(skill.contains(\"required policy\"), false);",
    ] {
        let source = format!(
            "let skill = std::fs::read_to_string(\"plugins/codexy/skills/demo/SKILL.md\")?;\n{assertion}"
        );
        assert_eq!(scan_source(&source).len(), 1, "{assertion}");
    }
}

#[test]
fn guard_rejects_assertion_macro_delimiters_and_contains_syntax_variants() {
    let guarded = "let skill = std::fs::read_to_string(\"plugins/codexy/skills/demo/SKILL.md\")?;\n";
    for assertion in [
        "assert!(skill.contains(\"required policy\"));",
        "assert! { skill.contains(\"required policy\") };",
        "assert! [ skill.contains(\"required policy\") ];",
        "debug_assert!(skill.contains(\"required policy\"));",
        "debug_assert! { skill.contains(\"required policy\") };",
        "debug_assert! [ skill.contains(\"required policy\") ];",
        "assert!(skill . contains(\"required policy\"));",
        "assert!(skill.\ncontains(\"required policy\"));",
        "assert!(skill.contains::<&str>(\"required policy\"));",
    ] {
        assert_eq!(scan_source(&format!("{guarded}{assertion}")).len(), 1, "{assertion}");
    }
}

#[test]
fn guard_rejects_function_pointer_turbofish_and_raw_contains_identifiers() {
    let guarded = "let skill = std::fs::read_to_string(\"plugins/codexy/skills/demo/SKILL.md\")?;\n";
    for assertion in [
        "assert!(skill.contains::<fn(char) -> bool>(char::is_whitespace));",
        "assert!(skill.contains::<Option<fn(char) -> Vec<bool>>>(None));",
        "assert!(skill.r#contains(\"required policy\"));",
        "assert!(skill.r#contains::<&str>(\"required policy\"));",
    ] {
        assert_eq!(scan_source(&format!("{guarded}{assertion}")).len(), 1, "{assertion}");
    }

    for assertion in [
        "assert!(skill.contains::<fn(char) -> bool>);",
        "assert!(skill.r#contains_more(\"required policy\"));",
    ] {
        assert!(scan_source(&format!("{guarded}{assertion}")).is_empty(), "{assertion}");
    }
}

#[test]
fn guard_ignores_custom_assertion_names() {
    for source in [
        "custom_assert!(snapshot.contains(\"heading\"));",
        "debug_assertion!(snapshot.contains(\"heading\"));",
        "assert_search_metadata(snapshot.contains(\"heading\"));",
        "πassert!(snapshot.contains(\"heading\"));",
    ] {
        assert!(scan_source(source).is_empty(), "{source}");
    }
}

#[test]
fn guard_allows_structured_exact_match_assertions_after_assert_prefixed_calls() {
    let source = concat!(
        "assert_search_metadata(&first, 1)?;\n",
        "assert_eq!(first[\"matches\"][0].as_str(), Some(\"ENTRY\"));\n",
    );
    assert!(scan_source(source).is_empty());
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

#[test]
fn guard_ignores_governed_paths_that_appear_only_in_comments() {
    let source = concat!(
        "// example path: plugins/codexy/skills/demo/SKILL.md\n",
        "let path = root.join(\"README.md\");\n",
        "let snapshot = std::fs::read_to_string(path)?;\n",
        "// structured-contract: non-contract substring rationale: verifies README heading output\n",
        "assert!(snapshot.contains(\"heading\"));"
    );
    assert!(scan_source(source).is_empty());
}
