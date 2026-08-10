use super::structured_contract_artifacts::TextShape;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn codexy_workflows_require_task_classification_first() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let orchestration = std::fs::read_to_string(
        root.join("plugins/codexy/skills/orchestration/SKILL.md"),
    )?;
    let classification = std::fs::read_to_string(
        root.join("plugins/codexy/skills/orchestration/references/task-classification.md"),
    )?;
    let git_workflow =
        std::fs::read_to_string(root.join("plugins/codexy/skills/git-workflow/SKILL.md"))?;
    let qa_prompt =
        std::fs::read_to_string(root.join("plugins/codexy/skills/qa/agents/openai.yaml"))?;
    let release_prompt = std::fs::read_to_string(
        root.join(".agents/skills/release-engineering/agents/openai.yaml"),
    )?;
    let plugin_prompt = std::fs::read_to_string(root.join("plugins/codexy/agents/openai.yaml"))?;

    TextShape::new(&orchestration).assert_required_concepts(
        "prompt-metadata.orchestration-classification-gate",
        &[
            "name orchestration",
            "must classify the lane through this skill before setup",
            "missing classification before setup validation release or other workflow actions",
        ],
    );
    TextShape::new(&classification).assert_required_concepts(
        "prompt-metadata.classification-output",
        &[
            "must classify first for any codexy work",
            "classification output",
            "lane type",
            "owner decision",
            "required skills",
            "required tools evidence",
            "lane relevant required evidence",
            "unavailable tool fallbacks",
            "first allowed action",
            "orchestration lane setup",
            "implementation",
            "review response",
            "github merge",
            "validation qa",
            "documentation skill authoring",
            "plugin release",
        ],
    );
    for (rule_id, surface, required) in [
        (
            "prompt-metadata.git-workflow-route",
            &git_workflow,
            &["$orchestration", "classification evidence"][..],
        ),
        ("prompt-metadata.qa-route", &qa_prompt, &["$orchestration"][..]),
        (
            "prompt-metadata.release-route",
            &release_prompt,
            &["$orchestration"][..],
        ),
        (
            "prompt-metadata.plugin-route",
            &plugin_prompt,
            &["$orchestration"][..],
        ),
    ] {
        TextShape::new(surface).assert_required_concepts(rule_id, required);
    }
    TextShape::new(&plugin_prompt).assert_required_concepts(
        "prompt-metadata.plugin-route-order",
        &["you must use $orchestration before setup"],
    );
    for surface in [
        &orchestration,
        &classification,
        &git_workflow,
        &qa_prompt,
        &release_prompt,
        &plugin_prompt,
    ] {
        TextShape::new(surface).assert_absent_concepts(
            "prompt-metadata.no-removed-skill-routes",
            &[
                "$task-classification",
                "$codex-orchestration",
                "$token-efficient-orchestration",
            ],
        );
    }
    Ok(())
}
