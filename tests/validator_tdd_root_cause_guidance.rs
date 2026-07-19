use std::path::Path;

use crate::support;

#[test]
fn tdd_skill_requires_root_cause_first_performance_repairs() {
    let skill = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/test-driven-development/SKILL.md"),
    )
    .expect("TDD skill");
    support::assert_structured_literals(
        &skill,
        "root-cause-first performance TDD contract",
        &[
            "Root-cause boundary:",
            "Harness cost:",
            "Integration target:",
            "Performance RED:",
            "MUST identify the root-cause boundary",
            "MUST place permutation cases at the pure or unit layer",
            "MUST keep one faithful boundary test",
            "A new standalone integration crate MUST document required isolation",
            "Performance RED MUST measure the original required workload exactly once",
            "MUST NOT satisfy performance acceptance with skips, filters, retries, sleeps, relaxed budgets, cache or runner upgrades as the sole fix, sharding alone, or a representative subset",
        ],
    );
}
