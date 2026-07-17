use std::{fs, path::Path};

use crate::paths::display_relative;
use crate::validation::orchestration_routing_semantics::{
    has_conflicting_luna_default, has_conflicting_sentinel_tier,
    has_conflicting_specialist_override, has_conflicting_tier_assignment,
};

mod assignments;
mod evidence;
mod policy;

use policy::{affirmative_field_values, policy_instructions, sections_for_heading};

const SKILL_PATH: &str = "skills/codex-orchestration/SKILL.md";
const RECIPIENT_ROUTING_HEADING: &str = "## Recipient Model Routing";
const DELIVERY_POLICY: &str = "Parent-to-generic-child delivery MUST pass `model: \"gpt-5.6-terra\"` and `thinking: \"high\"`; child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`.";
const ACTIVE_TIER_STARTS: &[&str] = &[
    "Root/orchestrator",
    "Generic implementation",
    "A named custom specialist",
    "`codexy-sentinel`",
    "`gpt-5.6-luna`",
];

const REQUIRED_BULLETS: &[(&str, &[&str], &str)] = &[
    (
        "Root/orchestrator: MUST use `gpt-5.6-sol`",
        &[],
        "root/orchestrator must use gpt-5.6-sol",
    ),
    (
        "Generic implementation, debugging, integration, and QA child thread: MUST",
        &["model: \"gpt-5.6-terra\"", "reasoning_effort: \"high\""],
        "generic child thread must explicitly request gpt-5.6-terra/high",
    ),
    (
        "`gpt-5.6-luna` is only for repository discovery, cataloging, simple",
        &[
            "documentation drafting, bounded polling, and repetitive checks.",
            "MUST NOT use Luna as the blanket default for implementation, security review, or ambiguous reasoning.",
        ],
        "Luna must stay limited to enumerated low-risk mechanical work",
    ),
    (
        "Cost guidance: Luna is an optimization for bounded low-risk work, not a",
        &["quality-neutral replacement for Terra."],
        "Luna cost guidance must reject quality-neutral replacement claims",
    ),
    (
        "A named custom specialist TOML is the model and reasoning-effort source of",
        &["truth. MUST NOT pass model or reasoning-effort overrides."],
        "named custom specialists must keep their TOML model and reasoning effort",
    ),
    (
        "`codexy-sentinel` remains `gpt-5.6-sol` / `xhigh`.",
        &[
            "MUST NOT use Ultra.",
            "Custom-agent invocations MUST use `fork_turns=\"none\"` or a positive bounded count with a self-contained handoff.",
        ],
        "codexy-sentinel must remain gpt-5.6-sol/xhigh and MUST NOT use Ultra",
    ),
];

const RECIPIENT_ROUTING_BULLETS: &[(&str, &[&str], &str)] = &[
    (
        "Configured UI model is authoritative; active child/parent thread ledger entries MUST",
        &[
            "record each destination owner's configured UI `model` and `thinking`",
            "separately from historical actual `turn_context` model and per-message overrides.",
        ],
        "active child/parent thread ledger must record the configured UI model and thinking",
    ),
    (
        "Every `send_message_to_thread` call, parent-to-child or child-to-parent, MUST",
        &[
            "explicitly pass the recipient's configured UI `model` and `thinking`.",
            "MUST NOT infer either from historical actual `turn_context` state, the sender, or ambient defaults.",
        ],
        "thread messages must explicitly pass the recipient model and thinking",
    ),
    (
        "Parent-to-generic-child delivery MUST pass",
        &[
            "`model: \"gpt-5.6-terra\"` and `thinking: \"high\"`",
            "child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`.",
        ],
        "parent-to-generic-child messages must use recipient gpt-5.6-terra/high; child-to-root messages must use recipient gpt-5.6-sol/high",
    ),
];

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    let path = plugin_root.join(SKILL_PATH);
    let Ok(skill) = fs::read_to_string(&path) else {
        return vec![format!(
            "{} could not be read for GPT-5.6 routing validation",
            display_relative(&path)
        )];
    };
    let routing_sections = sections_for_heading(&skill, "## GPT-5.6 Routing Matrix");
    if routing_sections.is_empty() {
        return vec![format!(
            "{} must define the GPT-5.6 routing matrix",
            display_relative(&path)
        )];
    }
    let routing_starts = REQUIRED_BULLETS
        .iter()
        .map(|(start, _, _)| *start)
        .chain(ACTIVE_TIER_STARTS.iter().copied())
        .collect::<Vec<_>>();
    let routing_bullets = routing_sections
        .iter()
        .map(|section| policy_instructions(section, &routing_starts))
        .collect::<Vec<_>>();
    let mut errors = routing_bullets
        .iter()
        .flat_map(|bullets| missing_required_bullets(&path, bullets, REQUIRED_BULLETS))
        .collect::<Vec<_>>();
    let recipient_sections = sections_for_heading(&skill, RECIPIENT_ROUTING_HEADING);
    if recipient_sections.is_empty() {
        errors.push(format!(
            "{} must define recipient model routing policy",
            display_relative(&path)
        ));
        return errors;
    }
    let recipient_starts = RECIPIENT_ROUTING_BULLETS
        .iter()
        .map(|(start, _, _)| *start)
        .chain(ACTIVE_TIER_STARTS.iter().copied())
        .chain(assignments::INSTRUCTION_STARTS.iter().copied())
        .chain(evidence::ROUTES.iter().map(|(marker, ..)| *marker))
        .collect::<Vec<_>>();
    let recipient_bullets = recipient_sections
        .iter()
        .flat_map(|section| policy_instructions(section, &recipient_starts))
        .collect::<Vec<_>>();
    errors.extend(missing_required_bullets(
        &path,
        &recipient_bullets,
        RECIPIENT_ROUTING_BULLETS,
    ));
    let delivery_assignments = assignments::delivery(&recipient_bullets, &recipient_starts);
    for (direction, model, error) in [
        (
            "parent-to-generic-child delivery must pass",
            "gpt-5.6-terra",
            "parent-to-generic-child messages must use recipient gpt-5.6-terra/high",
        ),
        (
            "child-to-root delivery must pass",
            "gpt-5.6-sol",
            "child-to-root messages must use recipient gpt-5.6-sol/high",
        ),
    ] {
        if delivery_assignments.iter().any(|(found, assignment)| {
            let models = affirmative_field_values(assignment, "model");
            let efforts = affirmative_field_values(assignment, "thinking");
            *found == direction
                && (!models.contains(&model)
                    || models.iter().any(|value| *value != model)
                    || !efforts.contains(&"high")
                    || efforts.iter().any(|value| *value != "high"))
        }) {
            errors.push(format!("{} {error}", display_relative(&path)));
        }
        if assignments::has_negated(&recipient_bullets, &recipient_starts, direction) {
            errors.push(format!("{} {error}", display_relative(&path)));
        }
    }
    for (conflict, message) in [
        (
            has_conflicting_specialist_override as fn(&str) -> bool,
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
        (
            has_conflicting_tier_assignment,
            "root/orchestrator must use gpt-5.6-sol; generic child thread must explicitly request gpt-5.6-terra/high",
        ),
        (
            has_conflicting_luna_default,
            "Luna must remain limited to bounded mechanical work",
        ),
        (
            has_conflicting_sentinel_tier,
            "codexy-sentinel must remain gpt-5.6-sol/xhigh",
        ),
    ] {
        if routing_bullets
            .iter()
            .flatten()
            .map(String::as_str)
            .chain(recipient_bullets.iter().filter_map(|bullet| {
                if bullet.starts_with("Captured #433 parent-to-generic-child evidence")
                    || bullet.starts_with("Reverse child-to-root evidence")
                {
                    None
                } else {
                    bullet
                        .strip_prefix(DELIVERY_POLICY)
                        .filter(|suffix| !suffix.trim().is_empty())
                        .or((!bullet.starts_with(DELIVERY_POLICY)).then_some(bullet))
                }
            }))
            .any(|bullet| conflict(bullet))
        {
            errors.push(format!("{} {message}", display_relative(&path)));
        }
    }
    for (marker, recipient, sender, thread, direction) in evidence::ROUTES {
        if evidence::invalid(&recipient_bullets, marker, recipient, sender, thread) {
            errors.push(format!(
                "{} {direction} evidence must pass recipient {recipient}/high",
                display_relative(&path)
            ));
        }
    }
    errors
}

fn missing_required_bullets(
    path: &Path,
    bullets: &[String],
    required: &[(&str, &[&str], &str)],
) -> Vec<String> {
    required
        .iter()
        .filter(|(start, clauses, _)| {
            let mut matches = bullets
                .iter()
                .filter(|bullet| required_clause_matches(bullet, start));
            matches.clone().next().is_none()
                || matches.any(|bullet| clauses.iter().any(|clause| !bullet.contains(clause)))
        })
        .map(|(_, _, error)| format!("{} {error}", display_relative(path)))
        .collect()
}

fn required_clause_matches(bullet: &str, prefix: &str) -> bool {
    bullet.starts_with(prefix)
        && (!prefix.ends_with("MUST") || !bullet[prefix.len()..].trim_start().starts_with("NOT"))
}
