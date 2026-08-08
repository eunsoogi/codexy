use std::{fs, path::Path};

const SKILL_PATH: &str = "skills/codex-orchestration/SKILL.md";
const ACTUAL_EVIDENCE: &str = r#"- Captured #433 parent-to-generic-child evidence: configured_ui_model="gpt-5.6-terra"; actual_turn_context_model="gpt-5.6-sol"; per_message_model="gpt-5.6-terra"; send_message_to_thread({ threadId: "child-433", model: "gpt-5.6-terra", thinking: "high" }).
- Reverse child-to-root evidence: configured_ui_model="gpt-5.6-sol"; actual_turn_context_model="gpt-5.6-terra"; per_message_model="gpt-5.6-sol"; send_message_to_thread({ threadId: "root-433", model: "gpt-5.6-sol", thinking: "medium" }).

"#;

struct Mutation {
    label: &'static str,
    needle: &'static str,
    replacement: &'static str,
    expected: &'static str,
    evidence: bool,
}

fn canonical_skill() -> String {
    fs::read_to_string(
        crate::paths::repository_root()
            .join("plugins/codexy")
            .join(SKILL_PATH),
    )
    .expect("canonical routing skill")
}

fn assert_exact_diagnostic(skill: &str, mutation: &Mutation) {
    let path = Path::new(SKILL_PATH);
    let errors = super::check_skill(path, skill);
    let expected = format!("{SKILL_PATH} {}", mutation.expected);
    assert!(
        errors.iter().any(|error| error == &expected),
        "{} missing exact diagnostic {expected:?}: {errors:#?}",
        mutation.label
    );
}

#[test]
fn cli_routing_matrix_preserves_exact_mutations_and_diagnostics() {
    let canonical = canonical_skill();
    let direct = [
        Mutation {
            label: "R1 root model",
            needle: "`gpt-5.6-sol` for decomposition",
            replacement: "`gpt-5.6-terra` for decomposition",
            expected: "root/orchestrator must use gpt-5.6-sol",
            evidence: false,
        },
        Mutation {
            label: "R2 generic child model",
            needle: "model: \"gpt-5.6-terra\"",
            replacement: "model: \"gpt-5.6-luna\"",
            expected: "generic child thread must explicitly request gpt-5.6-terra/high",
            evidence: false,
        },
        Mutation {
            label: "R3 specialist override",
            needle: "MUST NOT pass model or reasoning-effort overrides.",
            replacement: "MUST NOT pass model overrides.",
            expected: "named custom specialists must keep their TOML model and reasoning effort",
            evidence: false,
        },
        Mutation {
            label: "R4 sentinel tier",
            needle: "`codexy-sentinel` remains `gpt-5.6-sol` / `xhigh`. MUST NOT use Ultra",
            replacement: "`codexy-sentinel` remains `gpt-5.6-terra` / `ultra`. MUST NOT use Ultra",
            expected: "codexy-sentinel must remain gpt-5.6-sol/xhigh and MUST NOT use Ultra",
            evidence: false,
        },
        Mutation {
            label: "M1 recipient heading",
            needle: "## Recipient Model Routing",
            replacement: "## Message Routing",
            expected: "must define recipient model routing policy",
            evidence: false,
        },
        Mutation {
            label: "M2 configured ledger",
            needle: "destination owner's configured UI `model` and `thinking`",
            replacement: "destination owner's configured UI `thinking`",
            expected: "active child/parent thread ledger must record the configured UI model and thinking",
            evidence: false,
        },
        Mutation {
            label: "M3 recipient ledger",
            needle: "recipient's configured UI `model` and `thinking`",
            replacement: "recipient's configured UI `model`",
            expected: "thread messages must explicitly pass the recipient model and thinking",
            evidence: false,
        },
        Mutation {
            label: "M4 parent delivery model",
            needle: "Parent-to-generic-child delivery MUST pass `model: \"gpt-5.6-terra\"` and\n  `thinking: \"high\"`",
            replacement: "Parent-to-generic-child delivery MUST pass `model: \"gpt-5.6-sol\"` and\n  `thinking: \"high\"`",
            expected: "parent-to-generic-child messages must use recipient gpt-5.6-terra/high",
            evidence: false,
        },
        Mutation {
            label: "M5 child delivery model",
            needle: "child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"`\n  and `thinking: \"medium\"`",
            replacement: "child-to-root delivery MUST pass `model: \"gpt-5.6-terra\"`\n  and `thinking: \"high\"`",
            expected: "child-to-root messages must use recipient gpt-5.6-sol/medium",
            evidence: false,
        },
        Mutation {
            label: "M6 inference policy",
            needle: "MUST NOT\n  infer either from historical actual `turn_context` state, the sender, or ambient defaults.",
            replacement: "MUST\n  infer both from the sender's historical actual `turn_context` state.",
            expected: "thread messages must explicitly pass the recipient model and thinking",
            evidence: false,
        },
        Mutation {
            label: "D1 decoy parent evidence",
            needle: "Captured #433 parent-to-generic-child evidence: configured_ui_model=\"gpt-5.6-terra\"; actual_turn_context_model=\"gpt-5.6-sol\"; per_message_model=\"gpt-5.6-terra\"; send_message_to_thread({ threadId: \"child-433\", model: \"gpt-5.6-terra\", thinking: \"high\" })",
            replacement: "Captured #433 parent-to-generic-child evidence: not_configured_ui_model=\"gpt-5.6-terra\"; not_actual_turn_context_model=\"gpt-5.6-sol\"; not_per_message_model=\"gpt-5.6-terra\"; send_message_to_thread({ threadId: \"child-433\", model: \"gpt-5.6-sol\", recipient_model: \"gpt-5.6-terra\", model: \"gpt-5.6-terra\", thinking: \"low\", configured_thinking: \"high\", thinking: \"high\" })",
            expected: "parent-to-generic-child evidence must pass recipient gpt-5.6-terra/high",
            evidence: true,
        },
        Mutation {
            label: "D2 decoy child evidence",
            needle: "Reverse child-to-root evidence: configured_ui_model=\"gpt-5.6-sol\"; actual_turn_context_model=\"gpt-5.6-terra\"; per_message_model=\"gpt-5.6-sol\"; send_message_to_thread({ threadId: \"root-433\", model: \"gpt-5.6-sol\", thinking: \"medium\" })",
            replacement: "Reverse child-to-root evidence: not_configured_ui_model=\"gpt-5.6-sol\"; not_actual_turn_context_model=\"gpt-5.6-terra\"; not_per_message_model=\"gpt-5.6-sol\"; send_message_to_thread({ threadId: \"root-433\", model: \"gpt-5.6-terra\", recipient_model: \"gpt-5.6-sol\", model: \"gpt-5.6-sol\", thinking: \"low\", configured_thinking: \"high\", thinking: \"high\" })",
            expected: "child-to-root evidence must pass recipient gpt-5.6-sol/medium",
            evidence: true,
        },
        Mutation {
            label: "D3 inactive parent evidence",
            needle: "- Captured #433 parent-to-generic-child evidence:",
            replacement: "<!-- - Captured #433 parent-to-generic-child evidence:",
            expected: "parent-to-generic-child evidence must pass recipient gpt-5.6-terra/high",
            evidence: false,
        },
        Mutation {
            label: "D4 inactive child evidence",
            needle: "- Reverse child-to-root evidence:",
            replacement: "```text\n- Reverse child-to-root evidence:",
            expected: "child-to-root evidence must pass recipient gpt-5.6-sol/medium",
            evidence: false,
        },
    ];
    let evidence = [
        Mutation {
            label: "E1 missing parent model",
            needle: "model: \"gpt-5.6-terra\", thinking: \"high\"",
            replacement: "thinking: \"high\"",
            expected: "parent-to-generic-child evidence must pass recipient gpt-5.6-terra/high",
            evidence: true,
        },
        Mutation {
            label: "E2 missing parent thinking",
            needle: "model: \"gpt-5.6-terra\", thinking: \"high\"",
            replacement: "model: \"gpt-5.6-terra\"",
            expected: "parent-to-generic-child evidence must pass recipient gpt-5.6-terra/high",
            evidence: true,
        },
        Mutation {
            label: "E3 wrong parent model",
            needle: "model: \"gpt-5.6-terra\", thinking: \"high\"",
            replacement: "model: \"gpt-5.6-sol\", thinking: \"high\"",
            expected: "parent-to-generic-child evidence must pass recipient gpt-5.6-terra/high",
            evidence: true,
        },
        Mutation {
            label: "E4 missing child model",
            needle: "model: \"gpt-5.6-sol\", thinking: \"medium\"",
            replacement: "thinking: \"medium\"",
            expected: "child-to-root evidence must pass recipient gpt-5.6-sol/medium",
            evidence: true,
        },
        Mutation {
            label: "E5 missing child thinking",
            needle: "model: \"gpt-5.6-sol\", thinking: \"medium\"",
            replacement: "model: \"gpt-5.6-sol\"",
            expected: "child-to-root evidence must pass recipient gpt-5.6-sol/medium",
            evidence: true,
        },
        Mutation {
            label: "E6 wrong child model",
            needle: "model: \"gpt-5.6-sol\", thinking: \"medium\"",
            replacement: "model: \"gpt-5.6-terra\", thinking: \"medium\"",
            expected: "child-to-root evidence must pass recipient gpt-5.6-sol/medium",
            evidence: true,
        },
    ];

    for mutation in direct {
        let mutated = if mutation.evidence {
            let inserted = ACTUAL_EVIDENCE.replacen(mutation.needle, mutation.replacement, 1);
            canonical.replacen("## Read Next", &format!("{inserted}## Read Next"), 1)
        } else {
            canonical.replacen(mutation.needle, mutation.replacement, 1)
        };
        assert_ne!(
            canonical, mutated,
            "{} fixture mutation was absent",
            mutation.label
        );
        assert_exact_diagnostic(&mutated, &mutation);
    }
    for mutation in evidence {
        let inserted = ACTUAL_EVIDENCE.replacen(mutation.needle, mutation.replacement, 1);
        assert_ne!(
            ACTUAL_EVIDENCE, inserted,
            "{} evidence mutation was absent",
            mutation.label
        );
        let mutated = canonical.replacen("## Read Next", &format!("{inserted}## Read Next"), 1);
        assert_exact_diagnostic(&mutated, &mutation);
    }
}
