use crate::support::{copy_plugin_fixture, stderr, TestResult};

const REFERENCE: &str = "skills/git-workflow/references/review-response-clusters.md";
const RECEIPT_CREATE: &str =
    "1. [receipt-create] Before editing actionable review feedback, MUST create one typed JSON receipt.";
const RECEIPT_VALIDATE: &str =
    "2. [receipt-validate] Before implementation, MUST validate that exact receipt file with `scripts/validate-plugin-config --check-review-response-cluster --review-response-cluster-file receipt.json`.";
const CASE_EXCEPTION: &str =
    "3. [case-exception-prohibition] During repair, MUST NOT accept a case-specific exception as structural evidence.";
const REOPEN_EVIDENCE: &str =
    "4. [reopen-evidence-restriction] Non-reopened receipt states MUST NOT include reopen evidence.";
const FINAL_RECEIPT_VALIDATE: &str =
    "5. [final-receipt-validate] After addressing feedback and before push or handoff, MUST set the receipt state to repaired or reopened and validate that exact final-state file with `scripts/validate-plugin-config --check-review-response-cluster --review-response-cluster-file receipt.json`.";
const COMPLETE: &str = "## Required Procedure\n\n1. [receipt-create] Before editing actionable review feedback, MUST create one typed JSON receipt.\n2. [receipt-validate] Before implementation, MUST validate that exact receipt file with `scripts/validate-plugin-config --check-review-response-cluster --review-response-cluster-file receipt.json`.\n3. [case-exception-prohibition] During repair, MUST NOT accept a case-specific exception as structural evidence.\n4. [reopen-evidence-restriction] Non-reopened receipt states MUST NOT include reopen evidence.\n5. [final-receipt-validate] After addressing feedback and before push or handoff, MUST set the receipt state to repaired or reopened and validate that exact final-state file with `scripts/validate-plugin-config --check-review-response-cluster --review-response-cluster-file receipt.json`.\n\n## Typed Receipt\n";

#[test]
fn procedure_obligation_catalog_is_complete_and_normative() -> TestResult {
    assert_valid(COMPLETE)?;

    for obligation in [
        RECEIPT_CREATE,
        RECEIPT_VALIDATE,
        CASE_EXCEPTION,
        REOPEN_EVIDENCE,
        FINAL_RECEIPT_VALIDATE,
    ] {
        assert_rejected(&COMPLETE.replacen(&format!("{obligation}\n"), "", 1))?;
    }

    assert_rejected(&COMPLETE.replacen(
        "\n## Typed Receipt",
        &format!("\n{RECEIPT_CREATE}\n\n## Typed Receipt"),
        1,
    ))?;
    assert_rejected(&COMPLETE.replacen("[receipt-create]", "[unknown-obligation]", 1))?;
    assert_rejected(&COMPLETE.replacen("[receipt-create] ", "", 1))?;
    assert_rejected(&COMPLETE.replacen("MUST create", "MUST NOT create", 1))?;
    assert_rejected(&COMPLETE.replacen("MUST NOT accept", "MUST accept", 1))?;
    for (required, substituted) in [
        (
            "create one typed JSON receipt",
            "record one typed JSON receipt",
        ),
        (
            "--check-review-response-cluster",
            "--check-plugin-config",
        ),
        (
            "case-specific exception",
            "quoted feedback example",
        ),
        (
            "Non-reopened receipt states",
            "Reopened receipt states",
        ),
        (
            "before push or handoff",
            "after push or handoff",
        ),
    ] {
        assert_rejected(&COMPLETE.replacen(required, substituted, 1))?;
    }
    assert_rejected(&COMPLETE.replacen(
        "Before editing actionable review feedback",
        "Before implementation",
        1,
    ))?;
    assert_valid(&COMPLETE.replacen(" receipt.\n", " receipt:\n", 1))?;

    let reordered = format!(
        "## Required Procedure\n\n{FINAL_RECEIPT_VALIDATE}\n{REOPEN_EVIDENCE}\n{CASE_EXCEPTION}\n{RECEIPT_VALIDATE}\n{RECEIPT_CREATE}\n\nAdditional context is explanatory only.\n\n## Typed Receipt\n"
    );
    assert_valid(&reordered)?;
    assert_rejected(&format!("```markdown\n{COMPLETE}```\n"))?;
    assert_rejected(&format!("<!--\n{COMPLETE}-->\n"))?;
    let indented = COMPLETE
        .lines()
        .map(|line| format!("    {line}\n"))
        .collect::<String>();
    assert_rejected(&indented)?;
    assert_rejected(&COMPLETE.replacen(
        "## Required Procedure\n\n",
        "## Required Procedure\n\n# New top-level section\n\n",
        1,
    ))?;
    assert_rejected("## Required Procedure\n\n1. [receipt-create] Before editing, MUST create one typed JSON receipt.\n")?;
    Ok(())
}

fn assert_valid(procedure: &str) -> TestResult {
    let output = validate(procedure)?;
    assert!(output.status.success(), "unexpected failure: {}", stderr(&output));
    Ok(())
}

fn assert_rejected(procedure: &str) -> TestResult {
    let output = validate(procedure)?;
    assert!(
        !output.status.success(),
        "incomplete procedure unexpectedly passed: {procedure}"
    );
    assert!(
        stderr(&output).contains("review procedure"),
        "unexpected diagnostic: {}",
        stderr(&output)
    );
    Ok(())
}

fn validate(procedure: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    std::fs::write(plugin_root.join(REFERENCE), procedure)?;
    Ok(crate::support::validator_instruction_policy(&plugin_root)?)
}
