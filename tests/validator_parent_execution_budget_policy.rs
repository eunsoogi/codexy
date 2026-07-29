use std::fs;

#[path = "structured_contract.rs"]
mod structured_contract;
#[path = "structured_contract_rules/mod.rs"]
mod structured_contract_rules;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const PARENT_CLAUSES: &[&str] = &[
    "Every non-trivial parent-owned orchestration stage MUST declare finite implementation, repair, fanout, and reviewer-cycle limits before work begins.",
    "A parent-owned stage MUST NOT use more than three non-Sentinel specialists in total; the packaged Sentinel remains separate.",
    "A repeated parent helper or reviewer cycle MUST record either an explicit acceptance criterion newly satisfied or an existing blocker removed.",
    "Unchanged wait output and full-state replay MUST consume the parent-stage budget. They MUST NOT renew implementation, repair, fanout, or reviewer-cycle limits.",
    "A bounded thread-read fallback that returns oversized preview or history output MUST consume the current parent-stage budget and MUST record only bounded size and token metadata. It MUST NOT renew the stage.",
    "Parent-stage budget enforcement MUST preserve external-wait heartbeat semantics and the packaged Sentinel review gate.",
];

fn budget_path(plugin_root: &std::path::Path) -> std::path::PathBuf {
    plugin_root.join("skills/codex-orchestration/references/execution-budget.md")
}

fn copy_budget_fixture() -> TestResult<(tempfile::TempDir, std::path::PathBuf)> {
    Ok(support::copy_plugin_fixture_with_mutable_files(&[std::path::Path::new(
        "skills/codex-orchestration/references/execution-budget.md",
    )])?)
}

#[test]
fn validator_requires_parent_stage_fanout_and_reviewer_budgets() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(
        root.join("plugins/codexy/skills/codex-orchestration/references/execution-budget.md"),
    )?;
    structured_contract::assert_rules(
        &structured_contract::Contract::markdown(&source),
        structured_contract_rules::PARENT_EXECUTION_BUDGET,
    );

    for clause in PARENT_CLAUSES {
        let (_temp, plugin_root) = copy_budget_fixture()?;
        let path = budget_path(&plugin_root);
        let original = fs::read_to_string(&path)?;
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
    let (_temp, plugin_root) = copy_budget_fixture()?;
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
