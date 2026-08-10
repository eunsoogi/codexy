use crate::support;

use support::routing_validator::{TestResult, assert_policy_rejected, assert_rejected, validate};

#[test]
fn validator_requires_issue_549_candidate_routing_contract() -> TestResult {
    let skill = routing_skill()?;
    for (needle, expected) in [
        (
            "Generic implementation children MUST request `gpt-5.6-terra` with `reasoning_effort: \"high\"` as the fail-closed default. Promotion above Terra/high is allowed only as an explicit exception selected by complete validated measurement.",
            "generic child route must retain gpt-5.6-terra/high as the fail-closed default",
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
        "- Generic implementation children MUST request `gpt-5.6-terra` with `reasoning_effort: \"max\"` as the fail-closed default.",
        "gpt-5.6-terra/high",
    )
}

#[test]
fn validator_rejects_active_promotion_exceptions_without_complete_validation() -> TestResult {
    let skill = routing_skill()?;
    assert!(validate(skill.clone())?.is_empty(), "valid routing policy failed");
    for (addition, expected) in [
        (
            "- Promotion above Terra/high MAY proceed without complete validated measurement.\n",
            "promotion above Terra/high must remain an explicit exception selected by complete validated measurement",
        ),
        (
            "Promotion above Terra/high MUST be allowed after final acceptance alone.\n",
            "promotion above Terra/high must remain an explicit exception selected by complete validated measurement",
        ),
        (
            "1. Promotion above Terra/high MAY proceed only as an explicit exception before complete validated measurement.\n",
            "promotion above Terra/high must remain an explicit exception selected by complete validated measurement",
        ),
        (
            "Promotion above Terra/high MAY proceed before complete validated measurement only as an explicit exception selected by complete validated measurement.\n",
            "promotion above Terra/high must remain an explicit exception selected by complete validated measurement",
        ),
        (
            "Promotion above Terra/high MAY proceed prior to complete validated measurement only as an explicit exception selected by complete validated measurement.\n",
            "promotion above Terra/high must remain an explicit exception selected by complete validated measurement",
        ),
        (
            "Promotion above Terra/high MAY proceed without complete validated measurement, but operators cannot bypass audit controls.\n",
            "promotion above Terra/high must remain an explicit exception selected by complete validated measurement",
        ),
        (
            "Operators cannot apply overrides, while Generic implementation child Terra/high default MAY apply only while #549 remains open.\n",
            "generic child route must retain gpt-5.6-terra/high as the fail-closed default",
        ),
        (
            "Generic implementation child Terra/high default MAY apply only while #549 remains open.\n",
            "generic child route must retain gpt-5.6-terra/high as the fail-closed default",
        ),
        (
            "Generic implementation child Terra/high default MUST apply only while #549 remains open.\n",
            "generic child route must retain gpt-5.6-terra/high as the fail-closed default",
        ),
    ] {
        assert_policy_rejected(
            skill.replacen(
                "Generic implementation children MUST request `gpt-5.6-terra` with `reasoning_effort: \"high\"` as the fail-closed default. Promotion above Terra/high is allowed only as an explicit exception selected by complete validated measurement.",
                &format!("Generic implementation children MUST request `gpt-5.6-terra` with `reasoning_effort: \"high\"` as the fail-closed default. Promotion above Terra/high is allowed only as an explicit exception selected by complete validated measurement.\n{addition}"),
                1,
            ),
            expected,
        )?;
    }
    for prohibition in [
        "Promotion above Terra/high MUST NOT be allowed without complete validated measurement.\n",
        "1. Promotion above Terra/high MAY NOT proceed before complete validated measurement.\n",
        "Promotion above Terra/high MAY NEVER proceed before complete validated measurement.\n",
        "Promotion above Terra/high cannot proceed before complete validated measurement.\n",
        "Promotion above Terra/high MUST NOT be allowed without complete validated measurement, while reviewers MAY comment.\n",
        "Promotion above Terra/high MUST NOT be allowed without complete validated measurement, while reviewers MAY proceed.\n",
    ] {
        let errors = validate(skill.replacen(
            "## Recipient Model Routing",
            &format!("{prohibition}\n## Recipient Model Routing"),
            1,
        ))?;
        assert!(errors.is_empty(), "valid prohibition failed: {errors:#?}");
    }
    Ok(())
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
        "- Simple task MUST use model: \"gpt-5.6-luna\" with thinking: \"max\" even when high-risk.\n",
        "- Simple task MUST use Luna/max even when high-risk; it MUST NOT select Terra.\n",
        "- Simple task MAY use Luna/max even when high-risk.\n",
        "- Simple task MUST use Luna/max even when high-risk. Simple tasks MUST NOT use Luna/max for release work.\n",
        "- Simple task MUST use Luna/max even when high-risk and Simple tasks MUST NOT use Luna/max for release work.\n",
        "- Simple task MUST use Luna/max even when high-risk. simple tasks MUST NOT use Luna/max for release work.\n",
        "- sImPlE task MUST use Luna/max even when high-risk.\t  SIMPLE tasks MUST NOT use Luna/max for release work.\n",
        "- Simple task MUST use Luna/max even when high-risk and  simple tasks MUST NOT use Luna/max for release work.\n",
        "- Simple task MAY use Luna/max even when high-risk and\n  simple tasks MUST NOT use Luna/max for release work.\n",
        "- Simple task MAY use Luna/max even when high-risk but Simple tasks MUST NOT use Luna/max for release work.\n",
        "- Simple task MAY use Luna/max even when high-risk BUT\t sImPlE tasks MUST NOT use Luna/max for release work.\n",
        "- Simple tasks MUST NOT use Luna/max for release work but Simple task MAY use Luna/max even when high-risk.\n",
        "- Simple task MAY use Luna/max even when high-risk simple tasks MUST NOT use Luna/max for release work.\n",
        "- Simple task MAY use Luna/max even when high-risk but SIMPLE TASKS MUST NOT use Luna/max for release work.\n",
    ] {
        assert_policy_rejected(
            skill.replacen(simple_rule, &format!("{simple_rule}\n{addition}"), 1),
            "simple-work Luna/max candidates must require every bounded-work predicate",
        )?;
    }
    assert_policy_rejected(
        skill.replacen(
            simple_rule,
            &format!("{simple_rule}\n\nSimple task MUST use Luna/max even when high-risk.\n"),
            1,
        ),
        "simple-work Luna/max candidates must require every bounded-work predicate",
    )?;
    let errors = validate(skill.replacen(
        simple_rule,
        &format!("{simple_rule}\n- Simple tasks MUST NOT use Luna/max.\n"),
        1,
    ))?;
    assert!(errors.is_empty(), "Luna/max prohibition was an assignment: {errors:#?}");
    for subject in ["simple task", "SIMPLE TASK", "sImPlE TaSkS", "SIMPLE TASKS"] {
        assert_policy_rejected(
            skill.replacen(
                simple_rule,
                &format!(
                    "{simple_rule}\n- {subject} MAY use Luna/max even when high-risk but Simple tasks MUST NOT use Luna/max.\n"
                ),
                1,
            ),
            "simple-work Luna/max candidates must require every bounded-work predicate",
        )?;
        let errors = validate(skill.replacen(
            simple_rule,
            &format!("{simple_rule}\n- {subject} MUST NOT use Luna/max.\n"),
            1,
        ))?;
        assert!(errors.is_empty(), "standalone prohibition failed for {subject:?}: {errors:#?}");
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
