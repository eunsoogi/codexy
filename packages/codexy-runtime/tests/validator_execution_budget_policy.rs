use std::{fs, path::Path};

#[path = "structured_contract.rs"]
mod structured_contract;
use crate::support::{self, InstructionPolicyFixture};

use structured_contract::{Contract, Modality, Rule};

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
    "File churn MAY renew the budget.",
    "Diff churn MAY reset the budget.",
    "Test churn MAY renew the budget.",
    "Fingerprint churn MAY reset the budget.",
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
const MIXED_POLARITY_COUNTERMANDS: &[&str] = &[
    "Artifact churn MUST NOT renew the budget, but repeated wait refreshes MAY renew the budget.",
    "Artifact churn MUST NOT renew the budget, BUT repeated wait refreshes MAY renew the budget.",
    "Artifact churn MUST NOT renew the budget, BuT repeated wait refreshes MAY renew the budget.",
    "Artifact churn MUST NOT renew the budget, and file churn MAY renew the budget.",
    "Artifact churn MUST NOT renew the budget, while file churn MAY renew the budget.",
    "Artifact churn MUST NOT renew the budget but repeated wait refreshes MAY renew the budget.",
    "Artifact churn MUST NOT renew the budget and file churn MAY renew the budget.",
    "Artifact churn MUST NOT renew the budget while file churn MAY renew the budget.",
    "Artifact churn MUST NOT renew the budget but MAY reset it.",
];
const ADJACENT_MIXED_POLARITY_COUNTERMANDS: &[&str] = &[
    "Artifact churn MUST NOT renew the budget. File churn MAY renew the budget.",
    "Artifact churn MUST NOT renew the budget.\n## File churn MAY renew the budget.",
];

const EXECUTION_BUDGET: &str = "skills/codex-orchestration/references/execution-budget.md";

fn policy_fixture() -> TestResult<InstructionPolicyFixture> {
    Ok(support::instruction_policy_fixture(Path::new(EXECUTION_BUDGET))?)
}

fn reset_budget_file(fixture: &InstructionPolicyFixture) -> TestResult<(std::path::PathBuf, String)> {
    fixture.reset()?;
    let path = fixture.path().to_path_buf();
    Ok((path.clone(), fs::read_to_string(path)?))
}

#[test]
fn validator_requires_finite_execution_budget_contract() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let contract = fs::read_to_string(
        root.join("plugins/codexy/skills/codex-orchestration/references/execution-budget.md"),
    )?;
    Contract::markdown(&contract)
        .assert_rule(Rule::new(
            "execution-budget.child.finite-budget",
            "child lane",
            Modality::Required,
            &["declare"],
            &["finite execution budget"],
        ))
        .expect("execution-budget contract must require a finite child-lane budget");
    let fixture = policy_fixture()?;
    for clause in REQUIRED_CLAUSES {
        let (path, original) = reset_budget_file(&fixture)?;
        fs::write(
            &path,
            original.replace(clause, "removed execution-budget policy"),
        )?;

        let output = support::validator_instruction_policy_file(&path)?;
        assert!(!output.status.success(), "validator accepted {clause:?}");
        assert!(support::stderr(&output).contains("execution-budget contract"));
    }
    Ok(())
}

#[test]
fn validator_rejects_anchor_preserving_426_and_434_countermands() -> TestResult {
    let fixture = policy_fixture()?;
    for countermand in RENEWAL_COUNTERMANDS.iter().chain(OTHER_COUNTERMANDS) {
        let (path, original) = reset_budget_file(&fixture)?;
        let sequence = format!(
            "\n#426 sequence: a small adjacent edit, proof rerun, new edge, and changed fingerprint leave the same acceptance work.\n#434 sequence: repeated child waiting turns, goal refreshes, and polling occur without a material transition.\n{countermand}\n"
        );
        fs::write(&path, format!("{original}{sequence}"))?;

        let output = support::validator_instruction_policy_file(&path)?;
        assert!(
            !output.status.success(),
            "validator accepted countermanding #426/#434 policy {countermand:?}"
        );
        assert!(support::stderr(&output).contains("execution-budget contract"));
    }
    Ok(())
}

#[test]
fn validator_rejects_mixed_polarity_countermand() -> TestResult {
    let fixture = policy_fixture()?;
    for countermand in MIXED_POLARITY_COUNTERMANDS {
        let (path, original) = reset_budget_file(&fixture)?;
        fs::write(
            &path,
            format!(
                "{original}\n#426 sequence: a small adjacent edit, proof rerun, new edge, and changed fingerprint leave the same acceptance work.\n#434 sequence: repeated child waiting turns, goal refreshes, and polling occur without a material transition.\n{countermand}\n"
            ),
        )?;

        let output = support::validator_instruction_policy_file(&path)?;
        assert!(
            !output.status.success(),
            "validator accepted mixed-polarity countermand {countermand:?}"
        );
        assert!(support::stderr(&output).contains("execution-budget contract"));
    }
    Ok(())
}

#[test]
fn validator_rejects_adjacent_mixed_polarity_countermand() -> TestResult {
    let fixture = policy_fixture()?;
    for countermand in ADJACENT_MIXED_POLARITY_COUNTERMANDS {
        let (path, original) = reset_budget_file(&fixture)?;
        fs::write(&path, format!("{original}\n{countermand}\n"))?;

        let output = support::validator_instruction_policy_file(&path)?;
        assert!(
            !output.status.success(),
            "validator accepted adjacent mixed-polarity countermand {countermand:?}"
        );
        assert!(support::stderr(&output).contains("execution-budget contract"));
    }
    Ok(())
}

#[test]
fn validator_rejects_numbered_metadata_countermand() -> TestResult {
    let fixture = policy_fixture()?;
    let (path, original) = reset_budget_file(&fixture)?;
    fs::write(
        &path,
        format!("{original}\n#426 sequence: Artifact churn MAY renew the budget.\n"),
    )?;

    let output = support::validator_instruction_policy_file(&path)?;
    assert!(
        !output.status.success(),
        "validator accepted numbered metadata countermand"
    );
    assert!(support::stderr(&output).contains("execution-budget contract"));
    Ok(())
}

#[test]
fn validator_allows_benign_markdown_heading_and_comment() -> TestResult {
    let fixture = policy_fixture()?;
    let (path, original) = reset_budget_file(&fixture)?;
    fs::write(
        &path,
        format!(
            "{original}\n# Budget renewal details\n<!-- Artifact churn MAY renew the budget. -->\n"
        ),
    )?;

    let output = support::validator_instruction_policy_file(&path)?;
    assert!(
        output.status.success(),
        "validator rejected benign Markdown: {}",
        support::stderr(&output)
    );
    Ok(())
}

#[test]
fn validator_allows_multiline_html_comment() -> TestResult {
    let fixture = policy_fixture()?;
    let (path, original) = reset_budget_file(&fixture)?;
    fs::write(
        &path,
        format!("{original}\n<!--\nArtifact churn MAY renew the budget.\n-->\n"),
    )?;

    let output = support::validator_instruction_policy_file(&path)?;
    assert!(
        output.status.success(),
        "validator rejected a multiline HTML comment: {}",
        support::stderr(&output)
    );
    Ok(())
}

#[test]
fn validator_allows_convergent_progress_and_post_proof_termination() -> TestResult {
    let fixture = policy_fixture()?;
    let (path, original) = reset_budget_file(&fixture)?;
    fs::write(
        &path,
        format!(
            "{original}\nConvergent control: an explicit acceptance criterion was newly satisfied, required proof completed, and the lane terminates implementation.\n"
        ),
    )?;
    let output = support::validator_instruction_policy_file(&path)?;
    assert!(
        output.status.success(),
        "validator rejected convergent progress and post-proof termination: {}",
        support::stderr(&output)
    );
    Ok(())
}
