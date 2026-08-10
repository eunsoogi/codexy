use crate::support;

use support::routing_validator::{TestResult, assert_policy_rejected, validate};

#[test]
fn validator_requires_the_simple_route_to_be_one_affirmative_conjunction() -> TestResult {
    let skill = routing_skill()?;
    assert!(validate(skill.clone())?.is_empty(), "valid routing policy failed");
    let simple_rule = "Candidate simple work MUST use `gpt-5.6-luna` with `reasoning_effort: \"max\"` only when fixed scope, deterministic oracle, low-risk/reversible boundary, and no unresolved domain, security, permission, release, or ownership decision all hold.";
    for replacement in [
        simple_rule.replacen("only when", "even when", 1),
        simple_rule.replacen("fixed scope", "fixed scope, fixed scope", 1),
        simple_rule.replacen("deterministic oracle", "heuristic oracle", 1),
        simple_rule.replacen("low-risk/reversible boundary", "high-risk boundary", 1),
        simple_rule.replacen("no unresolved domain", "an unresolved domain", 1),
    ] {
        assert_policy_rejected(
            skill.replacen(simple_rule, &replacement, 1),
            "simple-work Luna/max candidates must require every bounded-work predicate",
        )?;
    }
    assert_policy_rejected(
        skill.replacen(
            "A matching named specialist MUST be selected before generic child routing;",
            "A matching named specialist MUST be selected after generic child routing;",
            1,
        ),
        "named specialist routing must precede generic child routing without TOML overrides",
    )
}

fn routing_skill() -> TestResult<String> {
    Ok(std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join("plugins/codexy/skills/orchestration/SKILL.md"),
    )?)
}
