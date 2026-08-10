use crate::support;
use std::path::Path;

const SKILL: &str = "skills/orchestration/SKILL.md";
const REFERENCES: [&str; 11] = [
    "references/task-classification.md",
    "references/classification-and-control.md",
    "references/goal-transition-reporting.md",
    "references/thread-and-worktree-routing.md",
    "references/orchestration-loop.md",
    "references/runtime-heartbeats.md",
    "references/parent-stop-preflight.md",
    "references/execution-budget.md",
    "references/token-efficient.md",
    "references/plain-language-user-replies.md",
    "references/natural-korean-responses.md",
];

pub(super) fn registered_orchestration_references() -> super::TestResult<Vec<String>> {
    let skill = std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join("plugins/codexy").join(SKILL),
    )?;
    parse_read_next_inventory(&skill)
}

fn parse_read_next_inventory(skill: &str) -> super::TestResult<Vec<String>> {
    let section = skill
        .split_once("## Read Next")
        .and_then(|(_, remainder)| remainder.split_once("## Classification Gate"))
        .map(|(section, _)| section)
        .ok_or("orchestration Read Next section")?;
    let parts = section.split('`').collect::<Vec<_>>();
    if parts.len() % 2 == 0 {
        return Err("Read Next contains an unmatched backtick".into());
    }
    let raw_references = parts
        .iter()
        .skip(1)
        .step_by(2)
        .copied()
        .collect::<Vec<_>>();
    if raw_references.len() != REFERENCES.len() {
        return Err("Read Next inventory is incomplete, duplicate, unknown, or retired".into());
    }
    let references = raw_references
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if references.len() != raw_references.len()
        || references.iter().any(|reference| !REFERENCES.contains(reference))
    {
        return Err("Read Next inventory is incomplete, duplicate, unknown, or retired".into());
    }
    Ok(references
        .into_iter()
        .map(|reference| format!("skills/orchestration/{reference}"))
        .collect())
}

pub(super) fn expected_references() -> Vec<String> {
    let mut references = REFERENCES
        .iter()
        .map(|reference| format!("skills/orchestration/{reference}"))
        .collect::<Vec<_>>();
    references.sort();
    references
}

#[test]
fn read_next_inventory_rejects_malformed_omitted_duplicate_unknown_and_retired_references()
-> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join("plugins/codexy").join(SKILL),
    )?;
    for (needle, replacement) in [
        (
            " and `references/classification-and-control.md`",
            "",
        ),
        (
            "`references/classification-and-control.md`",
            "`references/classification-and-control.md` and `references/classification-and-control.md`",
        ),
        (
            "`references/classification-and-control.md`",
            "`references/unknown.md`",
        ),
        (
            "`references/classification-and-control.md`",
            "`references/codex-orchestration.md`",
        ),
        (
            "`references/natural-korean-responses.md`",
            "`references/natural-korean-responses.md",
        ),
    ] {
        let fixture = support::plugin_fixture_with_mutable_files(&[Path::new(SKILL)])?;
        let path = fixture.root().join(SKILL);
        let mutated = source.replacen(needle, replacement, 1);
        assert_ne!(mutated, source, "fixture mutation must change the Read Next inventory");
        std::fs::write(path, mutated)?;

        let output = support::validator(fixture.root(), "--check-roles")?;
        assert!(!output.status.success());
    }
    Ok(())
}

#[test]
fn helper_rejects_duplicate_raw_occurrence_before_fixture_admission() -> super::TestResult {
    let source = std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join("plugins/codexy").join(SKILL),
    )?;
    let duplicate = source.replacen(
        "`references/classification-and-control.md`",
        "`references/classification-and-control.md` and `references/classification-and-control.md`",
        1,
    );
    assert_ne!(duplicate, source, "fixture mutation must create a raw duplicate");

    assert!(parse_read_next_inventory(&duplicate).is_err());
    Ok(())
}
