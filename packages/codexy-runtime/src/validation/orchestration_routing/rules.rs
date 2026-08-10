use super::clauses::boundaries;
use super::policy::policy_line;

pub(super) const ACTIVE_TIER_STARTS: &[&str] = &[
    "Root/orchestrator",
    "Generic implementation",
    "A matching named specialist",
    "Candidate simple work",
    "Candidate general work",
    "Promotion above Terra/high",
    "Ambiguous, high-risk, or incomplete",
    "A named custom specialist",
    "`codexy-sentinel`",
    "`gpt-5.6-luna`",
];

pub(super) const DELIVERY_POLICY: &str = "Parent-to-generic-child delivery MUST pass `model: \"gpt-5.6-terra\"` and `thinking: \"high\"`; child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"` and `thinking: \"medium\"`.";

const SIMPLE_ROUTE_PREFIX: &str =
    "Candidate simple work MUST use `gpt-5.6-luna` with `reasoning_effort: \"max\"` ";
const SIMPLE_ROUTE_CONJUNCTION: &str = "only when fixed scope, deterministic oracle, low-risk/reversible boundary, and no unresolved domain, security, permission, release, or ownership decision all hold.";

pub(super) fn simple_route_is_affirmative(bullet: &str) -> bool {
    bullet.strip_prefix(SIMPLE_ROUTE_PREFIX) == Some(SIMPLE_ROUTE_CONJUNCTION)
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum SimpleLunaAssignment {
    Affirmative,
    Prohibition,
}

pub(super) fn simple_luna_assignment(bullet: &str) -> Option<SimpleLunaAssignment> {
    let normalized = bullet.to_ascii_lowercase();
    let is_compact_luna_max = token_present(&normalized, "luna/max");
    let has_operands = is_compact_luna_max
        || token_present(&normalized, "gpt-5.6-luna")
            && ["reasoning_effort", "reasoning-effort", "thinking"]
                .into_iter()
                .any(|field| field_equals(&normalized, field, "max"));
    (normalized.contains("simple") && has_operands).then_some(())?;
    Some(if has_prohibition(&normalized) {
        SimpleLunaAssignment::Prohibition
    } else if ["must use", "may use", "should use", "can use"]
        .iter()
        .any(|modality| normalized.contains(modality))
    {
        SimpleLunaAssignment::Affirmative
    } else {
        return None;
    })
}

pub(super) fn luna_assignment_clauses(section: &str) -> Vec<String> {
    section
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start_matches(' ').trim_end();
            (!trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !trimmed.starts_with("![")
                && !trimmed.starts_with('<'))
            .then(|| policy_line(trimmed))
            .flatten()
        })
        .flat_map(|instruction| boundaries(instruction))
        .map(|clause| clause.trim().to_owned())
        .filter(|clause| !clause.is_empty())
        .collect()
}

fn has_prohibition(text: &str) -> bool {
    [
        "must not use",
        "may not use",
        "should not use",
        "cannot use",
        "can not use",
    ]
    .iter()
    .any(|modality| text.contains(modality))
}

fn token_present(text: &str, expected: &str) -> bool {
    text.match_indices(expected).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + expected.len()..].chars().next();
        before.is_none_or(|character| !character.is_ascii_alphanumeric() && character != '-')
            && after.is_none_or(|character| !character.is_ascii_alphanumeric() && character != '-')
    })
}

fn field_equals(text: &str, field: &str, expected: &str) -> bool {
    text.match_indices(field).any(|(start, _)| {
        let valid_start = text[..start]
            .chars()
            .next_back()
            .is_none_or(|character| !character.is_ascii_alphanumeric() && character != '_');
        let value = text[start + field.len()..]
            .trim_start()
            .strip_prefix(':')
            .map(str::trim_start)
            .unwrap_or_default()
            .trim_start_matches(['"', '`']);
        valid_start
            && value.strip_prefix(expected).is_some_and(|suffix| {
                suffix.is_empty() || suffix.starts_with(['"', '`', ' ', ',', ';', '.', ')'])
            })
    })
}

pub(super) const ROUTING_REQUIRED_BULLETS: &[(&str, &[&str], &str)] = &[
    (
        "Root/orchestrator: MUST use `gpt-5.6-sol`",
        &[],
        "root/orchestrator must use gpt-5.6-sol",
    ),
    (
        "Generic implementation children MUST request `gpt-5.6-terra`",
        &[
            "reasoning_effort: \"high\"` as the fail-closed default.",
            "Promotion above Terra/high is allowed only as an explicit exception selected by complete validated measurement.",
        ],
        "generic child route must retain gpt-5.6-terra/high as the fail-closed default",
    ),
    (
        "A matching named specialist MUST be selected before generic child routing;",
        &["its TOML remains authoritative."],
        "named specialist routing must precede generic child routing without TOML overrides",
    ),
    (
        "Candidate simple work MUST use `gpt-5.6-luna` with `reasoning_effort: \"max\"`",
        &[
            "fixed scope",
            "deterministic oracle",
            "low-risk/reversible boundary",
            "no unresolved domain, security, permission, release, or ownership decision",
        ],
        "simple-work Luna/max candidates must require every bounded-work predicate",
    ),
    (
        "Candidate general work MUST compare Terra/high, Terra/xhigh, and Terra/max",
        &["select the lowest effort meeting measured quality and economics gates."],
        "general-work candidates must compare Terra/high, Terra/xhigh, and Terra/max and select the lowest sufficient effort",
    ),
    (
        "Measurement gate: promotion above Terra/high MUST have zero P0/P1 defects,",
        &[
            "at least 95% acceptance",
            "either a five-point first-pass gain or 20% fewer repairs",
            "no more than 1.5x median cost or wall time.",
        ],
        "general-work promotion must meet explicit quality and economics thresholds",
    ),
    (
        "Ambiguous, high-risk, or incomplete classification MUST fail closed to root or a named specialist;",
        &["it MUST NOT select Luna."],
        "ambiguous, high-risk, or incomplete work must fail closed without Luna",
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

pub(super) const RECIPIENT_ROUTING_BULLETS: &[(&str, &[&str], &str)] = &[
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
        "The #417 reproduction MUST reject a create/send handoff that omits recipient `model` or `thinking`;",
        &["ambient or sender-derived Sol/medium is invalid."],
        "#417 recipient omissions must reject ambient or sender-derived Sol/medium",
    ),
    (
        "Parent-to-generic-child delivery MUST pass",
        &[
            "`model: \"gpt-5.6-terra\"` and `thinking: \"high\"`",
            "child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"` and `thinking: \"medium\"`.",
        ],
        "parent-to-generic-child messages must use recipient gpt-5.6-terra/high; child-to-root messages must use recipient gpt-5.6-sol/medium",
    ),
];
