use std::fs;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_allows_negated_countermand_examples() -> TestResult {
    let fixture = support::instruction_policy_fixture(std::path::Path::new(
        "skills/codex-orchestration/references/execution-budget.md",
    ))?;
    let path = fixture.path();
    let original = fs::read_to_string(&path)?;
    fs::write(
        &path,
        format!(
            "{original}\nThe statement \"Artifact churn MAY renew or reset the budget.\" MUST NOT be permitted.\nThe quoted text \"Artifact churn MUST NOT renew the budget but repeated wait refreshes MAY renew the budget.\" is illustrative.\nArtifact churn MUST NOT renew the budget buttermilk wait refreshes MAY renew the budget.\nArtifact churn MUST NOT renew the budget, but repeated wait refreshes MUST NOT renew the budget.\nArtifact churn MUST NOT renew the budget but repeated wait refreshes MUST NOT renew the budget.\nArtifact churn MUST NOT renew the budget but a reviewer MAY reset it.\nArtifact churn MUST NOT renew the budget. MAY reset it.\n"
        ),
    )?;

    let output = support::validator_instruction_policy_file(path)?;
    assert!(
        output.status.success(),
        "validator rejected a negated countermand example: {}",
        support::stderr(&output)
    );
    Ok(())
}
