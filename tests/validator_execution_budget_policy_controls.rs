use std::fs;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn budget_path(plugin_root: &std::path::Path) -> std::path::PathBuf {
    plugin_root.join("skills/codex-orchestration/references/execution-budget.md")
}

#[test]
fn validator_allows_negated_countermand_examples() -> TestResult {
    let (_temp, plugin_root) = support::copy_plugin_fixture_with_mutable_files(&[
        std::path::Path::new("skills/codex-orchestration/references/execution-budget.md"),
    ])?;
    let path = budget_path(&plugin_root);
    let original = fs::read_to_string(&path)?;
    fs::write(
        &path,
        format!(
            "{original}\nThe statement \"Artifact churn MAY renew or reset the budget.\" MUST NOT be permitted.\nThe quoted text \"Artifact churn MUST NOT renew the budget but repeated wait refreshes MAY renew the budget.\" is illustrative.\nArtifact churn MUST NOT renew the budget buttermilk wait refreshes MAY renew the budget.\nArtifact churn MUST NOT renew the budget, but repeated wait refreshes MUST NOT renew the budget.\nArtifact churn MUST NOT renew the budget but repeated wait refreshes MUST NOT renew the budget.\nArtifact churn MUST NOT renew the budget but a reviewer MAY reset it.\nArtifact churn MUST NOT renew the budget. MAY reset it.\n"
        ),
    )?;

    let output = support::validator_instruction_policy(&plugin_root)?;
    assert!(
        output.status.success(),
        "validator rejected a negated countermand example: {}",
        support::stderr(&output)
    );
    Ok(())
}
