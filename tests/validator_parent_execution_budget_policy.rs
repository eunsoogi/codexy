use std::fs;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const PARENT_CLAUSES: &[&str] = &[
    "Every non-trivial parent-owned orchestration stage MUST declare finite implementation, repair, fanout, and reviewer-cycle limits before work begins.",
    "A parent-owned stage MUST NOT use more than three non-Sentinel specialists in total; the packaged Sentinel remains separate.",
    "A repeated parent helper or reviewer cycle MUST record either an explicit acceptance criterion newly satisfied or an existing blocker removed.",
    "Unchanged wait output and full-state replay MUST consume the parent-stage budget and MUST NOT renew implementation, repair, fanout, or reviewer-cycle limits.",
    "A bounded thread-read fallback that returns oversized preview or history output MUST consume the current parent-stage budget, MUST record only bounded size and token metadata, and MUST NOT renew the stage.",
    "Parent-stage budget enforcement MUST preserve external-wait heartbeat semantics and the packaged Sentinel review gate.",
];

fn budget_path(plugin_root: &std::path::Path) -> std::path::PathBuf {
    plugin_root.join("skills/codex-orchestration/references/execution-budget.md")
}

#[test]
fn validator_requires_parent_stage_fanout_and_reviewer_budgets() -> TestResult {
    for clause in PARENT_CLAUSES {
        let (_temp, plugin_root) = support::copy_plugin_fixture()?;
        let path = budget_path(&plugin_root);
        let original = fs::read_to_string(&path)?;
        assert!(
            original.contains(clause),
            "missing parent budget clause {clause:?}"
        );
        fs::write(
            &path,
            original.replace(clause, "removed parent execution-budget policy"),
        )?;
        let output = support::validator_instruction_policy(&plugin_root)?;
        assert!(!output.status.success(), "validator accepted {clause:?}");
        assert!(support::stderr(&output).contains("execution-budget contract"));
    }
    Ok(())
}

#[test]
fn validator_rejects_parent_cycles_without_acceptance_progress() -> TestResult {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = budget_path(&plugin_root);
    let original = fs::read_to_string(&path)?;
    fs::write(
        &path,
        format!(
            "{original}\nA parent helper or reviewer cycle MAY repeat without acceptance progress or blocker removal.\n"
        ),
    )?;
    let output = support::validator_instruction_policy(&plugin_root)?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("execution-budget contract"));
    Ok(())
}
