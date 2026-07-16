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
const RENEWAL_COUNTERMANDS: &[&str] = &[
    "Artifact churn MAY renew or reset the budget.",
    "Artifact churn or a repeated wait refresh MAY renew the budget.",
    "artifact churn or repeated wait refreshes MAY reset the budget.",
    "A child MAY self-renew the budget from changed artifacts alone.",
    "Repeated wait refreshes MAY renew the budget.",
    "Artifact churn and a repeated wait refresh MAY renew the budget.",
];
const OTHER_COUNTERMANDS: &[&str] = &[
    "Budget exhaustion MAY call `update_goal(blocked)`.",
    "Repeated child waiting turns, goal refreshes, or polling MAY qualify as acceptance progress.",
];
const MIXED_POLARITY_COUNTERMAND: &str =
    "Artifact churn MUST NOT renew the budget, but repeated wait refreshes MAY renew the budget.";

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
fn validator_rejects_anchor_preserving_426_and_434_countermands() -> TestResult {
    for countermand in RENEWAL_COUNTERMANDS.iter().chain(OTHER_COUNTERMANDS) {
        let (_temp, plugin_root) = support::copy_plugin_fixture()?;
        let path = budget_path(&plugin_root);
        let original = fs::read_to_string(&path)?;
        let sequence = format!(
            "\n#426 sequence: a small adjacent edit, proof rerun, new edge, and changed fingerprint leave the same acceptance work.\n#434 sequence: repeated child waiting turns, goal refreshes, and polling occur without a material transition.\n{countermand}\n"
        );
        fs::write(&path, format!("{original}{sequence}"))?;

        let output = support::validator(&plugin_root, "--check")?;
        assert!(
            !output.status.success(),
            "validator accepted countermanding #426/#434 policy {countermand:?}"
        );
        assert!(support::stderr(&output).contains("execution-budget contract"));
    }
    Ok(())
}

#[test]
fn validator_allows_negated_countermand_examples() -> TestResult {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = budget_path(&plugin_root);
    let original = fs::read_to_string(&path)?;
    fs::write(
        &path,
        format!(
            "{original}\nThe statement \"Artifact churn MAY renew or reset the budget.\" MUST NOT be permitted.\n"
        ),
    )?;

    let output = support::validator(&plugin_root, "--check")?;
    assert!(
        output.status.success(),
        "validator rejected a negated countermand example: {}",
        support::stderr(&output)
    );
    Ok(())
}

#[test]
fn validator_rejects_mixed_polarity_countermand() -> TestResult {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = budget_path(&plugin_root);
    let original = fs::read_to_string(&path)?;
    fs::write(
        &path,
        format!(
            "{original}\n#426 sequence: a small adjacent edit, proof rerun, new edge, and changed fingerprint leave the same acceptance work.\n#434 sequence: repeated child waiting turns, goal refreshes, and polling occur without a material transition.\n{MIXED_POLARITY_COUNTERMAND}\n"
        ),
    )?;

    let output = support::validator(&plugin_root, "--check")?;
    assert!(
        !output.status.success(),
        "validator accepted mixed-polarity countermand"
    );
    assert!(support::stderr(&output).contains("execution-budget contract"));
    Ok(())
}

#[test]
fn validator_allows_convergent_progress_and_post_proof_termination() -> TestResult {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = budget_path(&plugin_root);
    let original = fs::read_to_string(&path)?;
    fs::write(
        &path,
        format!(
            "{original}\nConvergent control: an explicit acceptance criterion was newly satisfied, required proof completed, and the lane terminates implementation.\n"
        ),
    )?;
    let output = support::validator(&plugin_root, "--check")?;
    assert!(
        output.status.success(),
        "validator rejected convergent progress and post-proof termination: {}",
        support::stderr(&output)
    );
    Ok(())
}
