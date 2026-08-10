use crate::support;

use support::routing_validator::{TestResult, assert_policy_rejected, assert_rejected, validate};

#[test]
fn validator_requires_issue_549_candidate_routing_contract() -> TestResult {
    let skill = routing_skill()?;
    for (needle, expected) in [
        (
            "Until #549 merges, generic implementation children MUST request `gpt-5.6-terra` with `reasoning_effort: \"high\"`.",
            "current generic child route must remain gpt-5.6-terra/high until #549 merges",
        ),
        (
            "A matching named specialist MUST be selected before generic child routing; its TOML remains authoritative.",
            "named specialist routing must precede generic child routing without TOML overrides",
        ),
        (
            "Candidate simple work MUST use `gpt-5.6-luna` with `reasoning_effort: \"max\"` only when fixed scope, deterministic oracle, low-risk/reversible boundary, and no unresolved domain, security, permission, release, or ownership decision all hold.",
            "simple-work Luna/max candidates must require every bounded-work predicate",
        ),
        (
            "Candidate general work MUST compare Terra/high, Terra/xhigh, and Terra/max and select the lowest effort meeting measured quality and economics gates.",
            "general-work candidates must compare Terra/high, Terra/xhigh, and Terra/max and select the lowest sufficient effort",
        ),
        (
            "Measurement gate: promotion above Terra/high MUST have zero P0/P1 defects, at least 95% acceptance, either a five-point first-pass gain or 20% fewer repairs, and no more than 1.5x median cost or wall time.",
            "general-work promotion must meet explicit quality and economics thresholds",
        ),
        (
            "Ambiguous, high-risk, or incomplete classification MUST fail closed to root or a named specialist; it MUST NOT select Luna.",
            "ambiguous, high-risk, or incomplete work must fail closed without Luna",
        ),
        (
            "The #417 reproduction MUST reject a create/send handoff that omits recipient `model` or `thinking`; ambient or sender-derived Sol/medium is invalid.",
            "#417 recipient omissions must reject ambient or sender-derived Sol/medium",
        ),
    ] {
        let errors = validate(skill.replacen(needle, "", 1))?;
        assert!(
            errors.iter().any(|error| error.contains(expected)),
            "missing #549 diagnostic {expected:?} after removing {needle:?}: {errors:#?}"
        );
    }
    assert_rejected(
        "- Until #549 merges, generic implementation children MUST request `gpt-5.6-terra` with `reasoning_effort: \"max\"`.",
        "gpt-5.6-terra/high",
    )
}

#[test]
fn validator_requires_the_simple_route_to_be_one_affirmative_conjunction() -> TestResult {
    let skill = routing_skill()?;
    let errors = validate(skill.clone())?;
    assert!(errors.is_empty(), "valid routing policy failed: {errors:#?}");
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

#[test]
fn validator_rejects_every_additional_active_luna_max_simple_assignment() -> TestResult {
    let skill = routing_skill()?;
    let simple_rule = "Candidate simple work MUST use `gpt-5.6-luna` with `reasoning_effort: \"max\"` only when fixed scope, deterministic oracle, low-risk/reversible boundary, and no unresolved domain, security, permission, release, or ownership decision all hold.";
    for addition in [
        "- Simple-task candidate routing MUST use `gpt-5.6-luna` with `reasoning_effort: \"max\"` even when high-risk.\n",
        "- Simple-task candidate routing MUST use Luna/max even when high-risk.\n",
        "1. Simple task MUST use `gpt-5.6-luna` with `reasoning_effort: \"max\"` even when high-risk.\n",
        "- Container policy:\n  - Simple task MUST use `gpt-5.6-luna` with `reasoning_effort: \"max\"` even when high-risk.\n",
    ] {
        assert_policy_rejected(
            skill.replacen(simple_rule, &format!("{simple_rule}\n{addition}"), 1),
            "simple-work Luna/max candidates must require every bounded-work predicate",
        )?;
    }
    for inactive in [
        "```md\n- Simple task MUST use `gpt-5.6-luna` with `reasoning_effort: \"max\"` even when high-risk.\n```\n",
        "<!-- - Simple task MUST use `gpt-5.6-luna` with `reasoning_effort: \"max\"` even when high-risk. -->\n",
        "![Simple task MUST use gpt-5.6-luna with reasoning_effort max even when high-risk](image.png)\n",
        "<img alt=\"Simple task MUST use gpt-5.6-luna with reasoning_effort max even when high-risk\">\n",
    ] {
        let errors = validate(skill.replacen(simple_rule, &format!("{simple_rule}\n{inactive}"), 1))?;
        assert!(
            errors.is_empty(),
            "inactive content {inactive:?} was treated as active: {errors:#?}"
        );
    }
    Ok(())
}

fn routing_skill() -> TestResult<String> {
    Ok(std::fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join("plugins/codexy/skills/orchestration/SKILL.md"),
    )?)
}
