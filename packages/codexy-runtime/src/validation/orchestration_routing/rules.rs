pub(super) const ACTIVE_TIER_STARTS: &[&str] = &[
    "Root/orchestrator",
    "Generic implementation",
    "Until #549 merges",
    "A matching named specialist",
    "Candidate simple work",
    "Candidate general work",
    "Ambiguous, high-risk, or incomplete",
    "A named custom specialist",
    "`codexy-sentinel`",
    "`gpt-5.6-luna`",
];

pub(super) const DELIVERY_POLICY: &str = "Parent-to-generic-child delivery MUST pass `model: \"gpt-5.6-terra\"` and `thinking: \"high\"`; child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"` and `thinking: \"medium\"`.";

pub(super) const ROUTING_REQUIRED_BULLETS: &[(&str, &[&str], &str)] = &[
    (
        "Root/orchestrator: MUST use `gpt-5.6-sol`",
        &[],
        "root/orchestrator must use gpt-5.6-sol",
    ),
    (
        "Until #549 merges, generic implementation children MUST request `gpt-5.6-terra`",
        &["reasoning_effort: \"high\""],
        "current generic child route must remain gpt-5.6-terra/high until #549 merges",
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
