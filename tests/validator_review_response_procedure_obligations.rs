use crate::support::{copy_plugin_fixture, stderr, TestResult};

const REFERENCE: &str = "skills/git-workflow/references/review-response-clusters.md";
const RECEIPT_CREATE: &str =
    "1. [receipt-create] Before editing, MUST create one typed JSON receipt.";
const RECEIPT_VALIDATE: &str =
    "2. [receipt-validate] Before implementation, MUST validate that exact receipt file.";
const CASE_EXCEPTION: &str =
    "3. [case-exception-prohibition] During repair, MUST NOT accept a case-specific exception.";
const REOPEN_EVIDENCE: &str =
    "4. [reopen-evidence-restriction] Non-reopened receipt states MUST NOT include reopen evidence.";
const COMPLETE: &str = "## Required Procedure\n\n1. [receipt-create] Before editing, MUST create one typed JSON receipt.\n2. [receipt-validate] Before implementation, MUST validate that exact receipt file.\n3. [case-exception-prohibition] During repair, MUST NOT accept a case-specific exception.\n4. [reopen-evidence-restriction] Non-reopened receipt states MUST NOT include reopen evidence.\n\n## Typed Receipt\n";

#[test]
fn procedure_obligation_catalog_is_complete_and_normative() -> TestResult {
    assert_valid(COMPLETE)?;

    for obligation in [
        RECEIPT_CREATE,
        RECEIPT_VALIDATE,
        CASE_EXCEPTION,
        REOPEN_EVIDENCE,
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

    let reordered = format!(
        "## Required Procedure\n\n{REOPEN_EVIDENCE}\n{CASE_EXCEPTION}\n{RECEIPT_VALIDATE}\n{RECEIPT_CREATE}\n\nAdditional context is explanatory only.\n\n## Typed Receipt\n"
    );
    assert_valid(&reordered)?;
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
