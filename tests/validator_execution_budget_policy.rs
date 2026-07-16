use std::fs;

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const REQUIRED_CLAUSES: &[&str] = &[
    "Every non-trivial child lane MUST declare a finite execution budget before edits begin.",
    "The budget MUST name finite implementation, repair, and reviewer cycle limits.",
    "Continuation MUST consume budget and record either an explicit acceptance criterion newly satisfied or an existing blocker removed.",
    "File, diff, test, or fingerprint churn without reducing remaining acceptance work MUST NOT renew or reset the budget.",
    "A renewal MUST be an explicit parent-owned new finite budget with recorded acceptance progress or blocker removal.",
    "After all acceptance criteria and required proof are complete, the lane MUST terminate implementation; adjacent findings become non-blocking follow-up candidates.",
    "Budget exhaustion MUST produce one compact terminal parent handoff with current goal/plan, branch/worktree/HEAD, dirty inventory, proof, remaining criteria, and recommended next decision.",
    "Budget exhaustion MUST NOT call `update_goal(blocked)` and MUST NOT weaken external-gate heartbeat semantics.",
    "An external parent heartbeat MUST observe waiting state without messaging the child and MUST send one continuation only on a material transition.",
    "Repeated child waiting turns, goal refreshes, polling, duplicate narrative, unbounded reasoning, or status-only parent receipts MUST consume budget and MUST NOT qualify as acceptance progress.",
    "The execution-budget contract MUST apply to GPT-5.6 Terra child lanes while remaining model-agnostic and MUST NOT hard-code model-specific prose into the state machine.",
];
const NON_RESET_CLAUSE: &str = "File, diff, test, or fingerprint churn without reducing remaining acceptance work MUST NOT renew or reset the budget.";

fn budget_path(plugin_root: &std::path::Path) -> std::path::PathBuf {
    plugin_root.join("skills/codex-orchestration/references/execution-budget.md")
}

#[test]
fn validator_requires_finite_execution_budget_contract() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let contract = fs::read_to_string(
        root.join("plugins/codexy/skills/codex-orchestration/references/execution-budget.md"),
    )?;
    for clause in REQUIRED_CLAUSES {
        assert!(contract.contains(clause), "missing {clause:?}");
        let (_temp, plugin_root) = support::copy_plugin_fixture()?;
        let path = budget_path(&plugin_root);
        let original = fs::read_to_string(&path)?;
        fs::write(
            &path,
            original.replace(clause, "removed execution-budget policy"),
        )?;

        let output = support::validator(&plugin_root, "--check")?;
        assert!(!output.status.success(), "validator accepted {clause:?}");
        assert!(support::stderr(&output).contains("execution-budget contract"));
    }
    Ok(())
}

#[test]
fn validator_rejects_426_churn_and_repeated_wait_refreshes() -> TestResult {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = budget_path(&plugin_root);
    let original = fs::read_to_string(&path)?;
    let churn = "#426 sequence: small adjacent edit, proof rerun, new edge, changed fingerprint, same remaining acceptance work.";
    let repeated_wait = "#434 sequence: repeated child waiting turns, goal refreshes, and polling without a material transition.";
    fs::write(
        &path,
        original.replace(
            NON_RESET_CLAUSE,
            &format!("{churn}\n{repeated_wait}\nArtifact churn or a repeated wait refresh MAY renew the budget."),
        ),
    )?;

    let output = support::validator(&plugin_root, "--check")?;
    assert!(
        !output.status.success(),
        "validator accepted a changing-but-nonconvergent #426 or repeated wait refresh"
    );
    assert!(support::stderr(&output).contains("execution-budget contract"));

    fs::write(&path, original)?;
    let output = support::validator(&plugin_root, "--check")?;
    assert!(
        output.status.success(),
        "validator rejected the convergent post-proof termination control: {}",
        support::stderr(&output)
    );
    Ok(())
}
