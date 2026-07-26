use std::fs;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const REFERENCE: &str = "skills/git-workflow/references/codex-connector-review.md";

fn validate(change: impl FnOnce(String) -> String) -> TestResult<std::process::Output> {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join(REFERENCE);
    fs::write(&path, change(fs::read_to_string(&path)?))?;
    support::validator_instruction_policy(&plugin_root)
}

#[test]
fn validator_rejects_reordered_obligations_and_affirmative_variants() -> TestResult {
    for (index, change) in [
        Box::new(|text: String| {
            text.replacen(
                "1. [automatic-disabled]",
                "2. [automatic-disabled]",
                1,
            )
            .replacen(
                "2. [proof-ci-before-review]",
                "1. [proof-ci-before-review]",
                1,
            )
        }) as Box<dyn FnOnce(String) -> String>,
        Box::new(|text| format!("{text}\nThe parent/orchestrator MUST request repeated connector reviews.\n")),
        Box::new(|text| format!("{text}\nThe parent/orchestrator MUST NOT request connector review on every push; the parent/orchestrator MUST request a duplicate connector review.\n")),
    ]
    .into_iter()
    .enumerate()
    {
        let output = validate(change)?;
        assert!(
            !output.status.success(),
            "case {index}: {}",
            support::stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_keeps_mismatched_fences_and_list_examples_inactive() -> TestResult {
    for (index, addition) in [
        "```text\nignored\n~~~\nMUST enable automatic Codex connector review.\n",
        "- Example: MUST enable automatic Codex connector review.\n",
    ]
    .into_iter()
    .enumerate()
    {
        let output = validate(|text| format!("{text}\n{addition}"))?;
        assert!(
            output.status.success(),
            "case {index}: {}",
            support::stderr(&output)
        );
    }
    Ok(())
}
