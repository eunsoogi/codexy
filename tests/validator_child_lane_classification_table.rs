use std::process::Output;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;
    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

fn evidence(classification: &str) -> String {
    format!(
        "{classification}\nChild branch codexy/461-table was created after classification.\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n"
    )
}

fn assert_allowed(classification: &str) -> TestResult {
    let output = run_ownership_validator(&evidence(classification))?;
    assert!(
        output.status.success(),
        "expected classification table to pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn assert_rejected(classification: &str) -> TestResult {
    assert!(!run_ownership_validator(&evidence(classification))?.status.success());
    Ok(())
}

const ENGLISH_TABLE: &str = r#"| Task classification | Decision |
| --- | --- |
| Lane type | implementation |
| Secondary surfaces | workflow, validators |
| Owner decision | current-thread-owned implementation lane for #461 |
| Atomic scope | issue-sized |
| Required skills | task-classification, codex-orchestration, git-workflow |
| Required tools/evidence | goal, plan, codegraph, LSP, Sentinel |
| First allowed action | create branch after classification |
| Stop/blocker | None |"#;

#[test]
fn validator_allows_ordered_english_and_korean_classification_tables() -> TestResult {
    assert_allowed(ENGLISH_TABLE)?;
    let shallow_indent = ENGLISH_TABLE
        .lines()
        .map(|line| format!("   {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_allowed(&shallow_indent)?;
    for owner in [
        "current-thread-owned implementation lane; not parent-owned",
        "current-thread-owned implementation lane; no parent implementation edits",
        "current-thread-owned — 구현 소유자이며 부모 소유자가 아님",
        "current-thread-owned — 구현 소유자이며 부모 소유자가 아닙니다",
    ] {
        assert_allowed(&ENGLISH_TABLE.replace(
            "current-thread-owned implementation lane for #461",
            owner,
        ))?;
    }
    assert_allowed(
        r#"| Task classification | Decision |
| --- | --- |
| Lane type | 구현 |
| Secondary surfaces | 워크플로와 검증기 |
| Owner decision | current-thread-owned — 현재 작업이 구현을 소유함 |
| Atomic scope | 이슈 하나로 한정 |
| Required skills | task-classification, codex-orchestration |
| Required tools/evidence | 목표, 계획, 코드그래프, LSP, Sentinel |
| First allowed action | 분류를 마친 뒤 브랜치 생성 |
| Stop/blocker | 없음 |"#,
    )
}

#[test]
fn validator_rejects_missing_duplicate_malformed_and_legacy_shapes() -> TestResult {
    assert_rejected(&ENGLISH_TABLE.replace("| Atomic scope | issue-sized |\n", ""))?;
    assert_rejected(&ENGLISH_TABLE.replace(
        "| Atomic scope | issue-sized |",
        "| Atomic scope | issue-sized |\n| Atomic scope | duplicated |",
    ))?;
    assert_rejected(&ENGLISH_TABLE.replace("| --- | --- |", "| --- |"))?;
    assert_rejected(&format!(
        "{ENGLISH_TABLE}\n| Surprise field | silently accepted |"
    ))?;
    assert_rejected(&format!("{ENGLISH_TABLE}\nSurprise field | silently accepted"))?;
    assert_rejected(&format!("{ENGLISH_TABLE}\n\n{ENGLISH_TABLE}"))?;
    assert_rejected(
        r#"Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: current-thread-owned implementation lane for #461
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None"#,
    )
}

#[test]
fn validator_rejects_duplicate_table_after_same_lane_setup() -> TestResult {
    let evidence = format!(
        "Lane ownership: child-owned\n{ENGLISH_TABLE}\nChild branch codexy/461-table was created after classification.\n{ENGLISH_TABLE}\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n"
    );
    assert!(!run_ownership_validator(&evidence)?.status.success());
    Ok(())
}

#[test]
fn validator_rejects_setup_before_unprefixed_table() -> TestResult {
    let evidence = format!(
        "Child branch codexy/461-table was created before classification.\n{ENGLISH_TABLE}\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n"
    );
    assert!(!run_ownership_validator(&evidence)?.status.success());
    Ok(())
}

#[test]
fn validator_rejects_non_gfm_separator_cells() -> TestResult {
    assert_rejected(&ENGLISH_TABLE.replace("| --- | --- |", "| ::: | ::: |"))?;
    assert_rejected(&ENGLISH_TABLE.replace("| --- | --- |", "| ::---:: | ::---:: |"))
}

#[test]
fn validator_rejects_fenced_code_block_table() -> TestResult {
    assert_rejected(&format!("```text\n{ENGLISH_TABLE}\n```"))?;
    assert_rejected(&format!("   ~~~\n{ENGLISH_TABLE}\n   ~~~"))
}

#[test]
fn validator_rejects_indented_code_block_table() -> TestResult {
    let indented = ENGLISH_TABLE
        .lines()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_rejected(&indented)
}

#[test]
fn validator_rejects_html_comment_table() -> TestResult {
    assert_rejected(&format!("<!--\n{ENGLISH_TABLE}\n-->"))
}

#[test]
fn validator_rejects_negated_implementation_owner() -> TestResult {
    for owner in [
        "current-thread-owned reviewer; not implementation owner",
        "current-thread-owned implementation lane; not the implementation owner",
        "current-thread-owned implementation lane; does not own implementation",
        "current-thread-owned implementation lane; implementation ownership is absent",
        "current-thread-owned implementation lane; isn't the implementation owner",
        "current-thread-owned implementation lane; doesn't own implementation",
        "current-thread-owned 구현 lane; 구현 소유자가 아님",
        "current-thread-owned — 구현 소유자가 아닙니다",
        "current-thread-owned — 구현 소유자가 아니에요",
        "current-thread-owned implementation lane; reviewer only, not owner",
    ] {
        let classification = ENGLISH_TABLE.replace(
            "current-thread-owned implementation lane for #461",
            owner,
        );
        assert!(
            !run_ownership_validator(&evidence(&classification))?
                .status
                .success(),
            "contradictory owner must be rejected: {owner}"
        );
    }
    Ok(())
}


#[test]
fn validator_rejects_numbered_child_context_with_contradictory_owner() -> TestResult {
    let classification = ENGLISH_TABLE.replace(
        "current-thread-owned implementation lane for #461",
        "current-thread-owned implementation lane; does not own implementation",
    );
    let evidence = format!(
        "1. Lane ownership: child-owned\n{classification}\nChild branch codexy/461-table was created after classification.\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n"
    );
    assert!(!run_ownership_validator(&evidence)?.status.success());
    Ok(())
}

#[test]
fn packaged_prompts_and_templates_require_the_canonical_table() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let skill = std::fs::read_to_string(
        root.join("plugins/codexy/skills/task-classification/SKILL.md"),
    )?;
    let prompt = std::fs::read_to_string(
        root.join("plugins/codexy/skills/task-classification/agents/openai.yaml"),
    )?;
    let loop_template = std::fs::read_to_string(root.join(
        "plugins/codexy/skills/codex-orchestration/references/orchestration-loop.md",
    ))?;

    assert_eq!(
        skill
            .lines()
            .filter(|line| *line == "MUST emit exactly one ordered GFM table before taking the first workflow action:")
            .count(),
        1
    );
    assert_eq!(
        skill
            .lines()
            .filter(|line| *line == "| Task classification | Decision |")
            .count(),
        1
    );
    let prompt: serde_yaml::Value = serde_yaml::from_str(&prompt)?;
    assert_eq!(
        prompt["interface"]["default_prompt"].as_str(),
        Some("You MUST use $task-classification first and emit one ordered eight-row GFM table naming lane type, secondary surfaces, owner decision, atomic scope, required skills, required tools/evidence, first allowed action, and blocker before Codexy setup, delegation, implementation, PR, review-response, or merge work begins.")
    );
    assert_eq!(
        loop_template
            .lines()
            .filter(|line| *line == "| Task classification | Decision |")
            .count(),
        2
    );
    assert_eq!(
        loop_template
            .lines()
            .collect::<Vec<_>>()
            .windows(3)
            .filter(|lines| lines == &["Lane goal / success criteria:", "```", ""])
            .count(),
        1
    );
    assert_eq!(
        loop_template
            .lines()
            .collect::<Vec<_>>()
            .windows(3)
            .filter(|lines| lines == &["Worktree path:", "```", ""])
            .count(),
        1
    );
    Ok(())
}
