#[path = "structured_contract.rs"]
mod structured_contract;
#[path = "structured_contract_artifacts.rs"]
mod structured_contract_artifacts;
#[path = "structured_contract_rules/mod.rs"]
mod structured_contract_rules;

#[test]
fn token_efficient_orchestration_skill_preserves_proof_gates()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let token_skill = std::fs::read_to_string(
        root.join("plugins/codexy/skills/orchestration/SKILL.md"),
    )?;
    let prompt_yaml = std::fs::read_to_string(
        root.join("plugins/codexy/skills/orchestration/agents/openai.yaml"),
    )?;
    let template = std::fs::read_to_string(
        root.join("plugins/codexy/skills/orchestration/templates/delta-poll.md"),
    )?;
    let receipt = std::fs::read_to_string(root.join(
        "plugins/codexy/skills/orchestration/templates/session-audit-proof-receipt.json",
    ))?;
    let orchestration =
        std::fs::read_to_string(root.join("plugins/codexy/skills/orchestration/SKILL.md"))?;
    let transition = std::fs::read_to_string(root.join(
        "plugins/codexy/skills/orchestration/references/goal-transition-reporting.md",
    ))?;

    structured_contract::assert_rules(
        &structured_contract::Contract::markdown(&token_skill),
        structured_contract_rules::TOKEN_CONTAINMENT,
    );
    structured_contract::assert_rules(
        &structured_contract::Contract::markdown(&orchestration),
        structured_contract_rules::ORCHESTRATION,
    );
    structured_contract::assert_rules(
        &structured_contract::Contract::markdown(&transition),
        structured_contract_rules::TRANSITION,
    );

    let prompt = structured_contract_artifacts::Prompt::parse(&prompt_yaml)?;
    assert_eq!(prompt.display_name(), "Token-Efficient Orchestration");
    assert!(prompt.allow_implicit_invocation());
    structured_contract::assert_rules(
        &structured_contract::Contract::markdown(prompt.default_prompt()),
        structured_contract_rules::TOKEN_PROMPT,
    );
    structured_contract_artifacts::TextShape::new(prompt.default_prompt())
        .assert_absent_inflections("token.prompt.no-polling-language", &["poll"]);
    structured_contract_artifacts::TextShape::new(&token_skill).assert_absent_concepts(
        "token.skill.no-stale-version-or-review-gate",
        &["installed Codexy plugin is version 1.1.0", "Codex review"],
    );
    structured_contract_artifacts::TextShape::new(&template)
        .assert_absent_concepts("token.delta-template.no-review-gate", &["Codex review"]);

    structured_contract_artifacts::Template::parse(&template).assert_slots(
        "token.delta-template.required-slots",
        &[
            "head SHA",
            "base SHA",
            "event id",
            "event kind",
            "changed ids",
            "stale or demoted",
            "external gate wait",
            "parent delta before transition",
            "heartbeat automation id",
            "target thread",
            "bounded schedule",
            "state fingerprint",
            "BLOCK receipt",
            "repair plan",
            "faithful RED coverage",
            "terminal proof",
            "fresh Sentinel review for new file state or head",
            "archive candidates inspected",
            "active reservation ledger",
            "archive decision",
            "unresolved review thread ids and outdated status",
            "child owner evidence",
            "merge readiness or stop condition",
            "one next action",
        ],
    );

    let receipt = structured_contract_artifacts::JsonShape::parse(&receipt)?;
    receipt.assert_bool("token.audit-receipt.metadata-only", "/metadataOnly", true);
    receipt.assert_paths(
        "token.audit-receipt.required-fields",
        &[
            "/audit/inputSha256",
            "/audit/observationalOnly",
            "/audit/comparison/ownerBoundary",
            "/audit/comparison/windowPolicy",
            "/audit/comparison/before/inputSha256",
            "/audit/comparison/before/sessionId",
            "/audit/comparison/after/inputSha256",
            "/audit/comparison/after/sessionId",
            "/audit/ownerTreeSessions",
            "/audit/ownerTreeSessions/0/inputSha256",
            "/audit/ownerTreeTotals/sessionCount",
            "/audit/ownerTreeTotals/recordsObserved",
            "/audit/ownerTreeTotals/turnEvents",
            "/audit/ownerTreeTotals/cumulativeTokens",
            "/audit/ownerTreeTotals/toolInputBytes",
            "/audit/ownerTreeTotals/toolOutputBytes",
            "/audit/ownerTreeTotals/execFamily/calls",
            "/audit/ownerTreeTotals/execFamily/inputBytes",
            "/audit/ownerTreeTotals/execFamily/outputBytes",
            "/audit/ownerTreeTotals/waitFamily/calls",
            "/audit/ownerTreeTotals/waitFamily/inputBytes",
            "/audit/ownerTreeTotals/waitFamily/outputBytes",
            "/metrics/reviewFeedback",
            "/metrics/childAgeSeconds",
            "/metrics/retriesByKind",
            "/goalPlanReceipts",
            "/helpers",
            "/commandReceipts",
            "/commandReceipts/0/commandId",
            "/commandReceipts/0/argumentsRedacted",
            "/installed/cacheRootRelative",
        ],
    );
    receipt.assert_absent_paths(
        "token.audit-receipt.no-review-request",
        &["/reviewRequests", "/reviewRequest"],
    );
    Ok(())
}
