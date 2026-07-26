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

fn reorder_first_two_obligations(text: String) -> String {
    let mut lines = text.lines().map(str::to_owned).collect::<Vec<_>>();
    let first = lines
        .iter()
        .position(|line| line.starts_with("1. [automatic-disabled]"))
        .expect("first obligation");
    let second = lines
        .iter()
        .position(|line| line.starts_with("2. [proof-ci-before-review]"))
        .expect("second obligation");
    lines.swap(first, second);
    format!("{}\n", lines.join("\n"))
}

#[test]
fn validator_rejects_reordered_obligations_and_affirmative_variants() -> TestResult {
    for (index, change) in [
        Box::new(reorder_first_two_obligations) as Box<dyn FnOnce(String) -> String>,
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
fn validator_keeps_modal_subjects_when_separating_must_clauses() -> TestResult {
    for (index, addition) in [
        "Automatic Codex connector review MUST be enabled.",
        "Automatic Codex connector review MUST NOT be enabled; Automatic Codex connector review MUST be enabled.",
    ]
    .into_iter()
    .enumerate()
    {
        let output = validate(|text| format!("{text}\n{addition}\n"))?;
        assert!(
            !output.status.success(),
            "affirmative case {index}: {}",
            support::stderr(&output)
        );
    }

    let output = validate(|text| {
        format!("{text}\nAutomatic Codex connector review MUST NOT be enabled.\n")
    })?;
    assert!(
        output.status.success(),
        "negative control: {}",
        support::stderr(&output)
    );
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
