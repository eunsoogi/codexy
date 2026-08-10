use crate::support;

use support::routing_validator::{TestResult, validate};

const PROMOTION_ERROR: &str =
    "promotion above Terra/high must remain an explicit exception selected by complete validated measurement";
const DEFAULT_ERROR: &str = "generic child route must retain gpt-5.6-terra/high as the fail-closed default";

#[test]
fn validator_binds_promotion_and_default_operands_to_subclauses() -> TestResult {
    let skill = routing_skill()?;
    for addition in [
        "Promotion above Terra/high MAY proceed after final acceptance alone, while reviewers are allowed only as an explicit exception selected by complete validated measurement.",
        "Promotion above Terra/high, while #549 remains open, MAY proceed after final acceptance alone.",
        "- Reviewers are allowed only as an explicit exception selected by complete validated measurement, while Promotion above Terra/high MAY proceed after final acceptance alone.",
        "- Reviewers are allowed only as an explicit exception selected by complete validated measurement while #549 remains open, and Promotion above Terra/high MAY proceed after final acceptance alone.",
        "- Reviewers are allowed only as an explicit exception selected by complete validated measurement while #549 remains open, or Promotion above Terra/high MAY proceed after final acceptance alone.",
        "Promotion above Terra/high is allowed only as an explicit exception selected by complete validated measurement! Promotion above Terra/high MAY proceed after final acceptance alone.",
        "Promotion above Terra/high is allowed only as an explicit exception selected by complete validated measurement? Promotion above Terra/high MAY proceed after final acceptance alone.",
    ] {
        let errors = validate(with_routing_addition(&skill, addition))?;
        assert!(
            errors.iter().any(|error| error.contains(PROMOTION_ERROR)),
            "promotion bypass {addition:?} passed: {errors:#?}"
        );
    }
    for control in [
        "Promotion above Terra/high MUST NOT be allowed without complete validated measurement, while reviewers MAY proceed.",
        "Promotion above Terra/high MUST NOT be allowed without complete validated measurement or reviewers MAY proceed.",
        "Promotion above Terra/high MAY proceed only as an explicit exception selected by complete validated measurement and requires no additional reviewer action.",
        "Generic implementation child Terra/high default MUST apply generally, while reviewers MAY apply only when #549 remains open.",
    ] {
        let errors = validate(with_routing_addition(&skill, control))?;
        assert!(
            errors.is_empty(),
            "separate-subject control {control:?} failed: {errors:#?}"
        );
    }
    assert!(
        validate(with_routing_addition(
            &skill,
            "Generic implementation child Terra/high default MAY apply only when #549 remains open.",
        ))?
        .iter()
        .any(|error| error.contains(DEFAULT_ERROR))
    );
    assert!(
        validate(with_routing_addition(
            &skill,
            "Generic implementation child Terra/high default, while #549 remains open, MAY apply generally.",
        ))?
        .iter()
        .any(|error| error.contains(DEFAULT_ERROR))
    );
    assert!(
        validate(with_routing_addition(
            &skill,
            "Generic implementation child Terra/high default MAY apply generally, while #549 remains open and reviewers MAY apply.",
        ))?
        .iter()
        .any(|error| error.contains(DEFAULT_ERROR))
    );
    Ok(())
}

fn with_routing_addition(skill: &str, addition: &str) -> String {
    skill.replacen(
        "## Recipient Model Routing",
        &format!("{addition}\n## Recipient Model Routing"),
        1,
    )
}

fn routing_skill() -> TestResult<String> {
    Ok(std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join("plugins/codexy/skills/orchestration/SKILL.md"),
    )?)
}
