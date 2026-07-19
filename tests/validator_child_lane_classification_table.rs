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
        "Lane ownership: child-owned\n{classification}\nChild branch codexy/461-table was created after classification.\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n"
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

    assert!(skill.contains("MUST emit exactly one ordered GFM table"));
    assert!(skill.contains("Values MAY be localized"));
    assert_eq!(skill.matches("| Task classification | Decision |").count(), 1);
    assert!(prompt.contains("one ordered eight-row GFM table"));
    assert_eq!(
        loop_template
            .matches("| Task classification | Decision |")
            .count(),
        2
    );
    assert!(loop_template.contains("Lane goal / success criteria:\n```\n\n| Task classification"));
    assert!(loop_template.contains("Worktree path:\n```\n\n| Task classification"));
    Ok(())
}
