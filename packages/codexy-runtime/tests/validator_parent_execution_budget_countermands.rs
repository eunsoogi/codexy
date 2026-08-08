use std::fs;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn budget_fixture() -> TestResult<support::InstructionPolicyFixture> {
    Ok(support::instruction_policy_fixture(std::path::Path::new(
        "skills/codex-orchestration/references/execution-budget.md",
    ))?)
}

#[test]
fn validator_rejects_repeated_parent_cycles_without_criterion_or_blocker_progress() -> TestResult {
    for clause in [
        "A parent reviewer cycle MAY be repeated without a newly satisfied acceptance criterion or a removed blocker.",
        "A parent helper cycle MAY repeat without acceptance criterion satisfaction or blocker removal.",
    ] {
        let fixture = budget_fixture()?;
        let path = fixture.path();
        let original = fs::read_to_string(&path)?;
        fs::write(&path, format!("{original}\n{clause}\n"))?;
        let output = support::validator_instruction_policy_file(path)?;
        assert!(!output.status.success(), "validator accepted {clause:?}");
    }
    Ok(())
}

#[test]
fn parent_cycle_countermand_keeps_negated_commented_and_sentence_boundary_controls() -> TestResult {
    for clause in [
        "A parent reviewer cycle MUST NOT be repeated without a newly satisfied acceptance criterion or a removed blocker.",
        "A parent reviewer cycle MUST NOT repeat if no acceptance criterion is newly satisfied, nor is a blocker removed.",
        "<!-- A parent helper cycle MAY repeat without acceptance criterion satisfaction or blocker removal. -->",
        "A parent reviewer cycle MAY repeat with a newly satisfied acceptance criterion. Without a blocker, it stops.",
    ] {
        let fixture = budget_fixture()?;
        let path = fixture.path();
        let original = fs::read_to_string(&path)?;
        fs::write(&path, format!("{original}\n{clause}\n"))?;
        let output = support::validator_instruction_policy_file(path)?;
        assert!(output.status.success(), "validator rejected {clause:?}");
    }
    Ok(())
}

#[test]
fn parent_cycle_countermand_preserves_alternate_progress_and_rejects_clause_variants() -> TestResult
{
    let valid = [
        "A parent reviewer cycle MAY repeat without a newly satisfied acceptance criterion when an existing blocker is removed.",
        "A parent reviewer cycle MAY repeat if an existing blocker is removed, even if no acceptance criterion is newly satisfied.",
        "A parent reviewer cycle MAY repeat if an existing blocker is removed, but no acceptance criterion is newly satisfied.",
        "A parent reviewer cycle MAY repeat without a newly satisfied acceptance criterion but an existing blocker is removed.",
    ];
    let invalid = [
        "A parent reviewer cycle MAY repeat even if no acceptance criterion is newly satisfied and no blocker is removed.",
        "A parent reviewer cycle MAY repeat without acceptance criterion or blocker removal and MUST NOT retain stale review bodies.",
        "A parent reviewer cycle MAY repeat with no newly satisfied acceptance criterion and no blocker removed.",
        "A parent reviewer cycle MAY repeat if no acceptance criterion is newly satisfied, nor is a blocker removed.",
        "A parent reviewer cycle MAY repeat without a newly satisfied acceptance criterion, but no blocker is removed.",
        "A parent reviewer cycle MAY repeat without a newly satisfied acceptance criterion but no blocker is removed.",
    ];

    let fixture = budget_fixture()?;
    let path = fixture.path();
    let original = fs::read_to_string(&path)?;
    for clause in valid {
        fs::write(&path, format!("{original}\n{clause}\n"))?;
        assert!(
            support::validator_instruction_policy_file(path)?
                .status
                .success(),
            "validator rejected alternate blocker progress {clause:?}"
        );
    }

    for clause in invalid {
        fs::write(&path, format!("{original}\n{clause}\n"))?;
        assert!(
            !support::validator_instruction_policy_file(path)?
                .status
                .success(),
            "validator accepted {clause:?}"
        );
    }
    Ok(())
}

#[test]
fn parent_cycle_countermand_rejects_continuation_verbs_without_progress() -> TestResult {
    let valid = [
        "A parent reviewer cycle MAY continue when an existing blocker is removed.",
        "A parent reviewer MAY continue reviewing evidence without another cycle.",
        "The quoted text \"A parent reviewer cycle MAY continue without acceptance criterion satisfaction or blocker removal.\" is illustrative.",
    ];
    let invalid = [
        "A parent reviewer cycle MAY continue without acceptance criterion satisfaction or blocker removal.",
        "A parent helper cycle MAY continue even if no acceptance criterion is newly satisfied and no blocker is removed.",
    ];

    let fixture = budget_fixture()?;
    let path = fixture.path();
    let original = fs::read_to_string(&path)?;
    for clause in valid {
        fs::write(&path, format!("{original}\n{clause}\n"))?;
        assert!(
            support::validator_instruction_policy_file(path)?
                .status
                .success(),
            "validator rejected allowed continuation {clause:?}"
        );
    }
    for clause in invalid {
        fs::write(&path, format!("{original}\n{clause}\n"))?;
        assert!(
            !support::validator_instruction_policy_file(path)?
                .status
                .success(),
            "validator accepted continuation countermand {clause:?}"
        );
    }
    Ok(())
}
