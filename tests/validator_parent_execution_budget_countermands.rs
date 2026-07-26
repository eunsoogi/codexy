use std::fs;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn budget_path(plugin_root: &std::path::Path) -> std::path::PathBuf {
    plugin_root.join("skills/codex-orchestration/references/execution-budget.md")
}

#[test]
fn validator_rejects_repeated_parent_cycles_without_criterion_or_blocker_progress() -> TestResult {
    for clause in [
        "A parent reviewer cycle MAY be repeated without a newly satisfied acceptance criterion or a removed blocker.",
        "A parent helper cycle MAY repeat without acceptance criterion satisfaction or blocker removal.",
    ] {
        let (_temp, plugin_root) = support::copy_plugin_fixture()?;
        let path = budget_path(&plugin_root);
        let original = fs::read_to_string(&path)?;
        fs::write(&path, format!("{original}\n{clause}\n"))?;
        let output = support::validator_instruction_policy(&plugin_root)?;
        assert!(!output.status.success(), "validator accepted {clause:?}");
    }
    Ok(())
}

#[test]
fn parent_cycle_countermand_keeps_negated_commented_and_sentence_boundary_controls() -> TestResult {
    for clause in [
        "A parent reviewer cycle MUST NOT be repeated without a newly satisfied acceptance criterion or a removed blocker.",
        "<!-- A parent helper cycle MAY repeat without acceptance criterion satisfaction or blocker removal. -->",
        "A parent reviewer cycle MAY repeat with a newly satisfied acceptance criterion. Without a blocker, it stops.",
    ] {
        let (_temp, plugin_root) = support::copy_plugin_fixture()?;
        let path = budget_path(&plugin_root);
        let original = fs::read_to_string(&path)?;
        fs::write(&path, format!("{original}\n{clause}\n"))?;
        let output = support::validator_instruction_policy(&plugin_root)?;
        assert!(output.status.success(), "validator rejected {clause:?}");
    }
    Ok(())
}
