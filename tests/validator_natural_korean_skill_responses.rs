use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SHARED_GUIDANCE: &str =
    "plugins/codexy/skills/codex-orchestration/references/natural-korean-responses.md";

const REPRESENTATIVE_SKILLS: [&str; 4] = [
    "plugins/codexy/skills/codex-orchestration/SKILL.md",
    "plugins/codexy/skills/debugging/SKILL.md",
    "plugins/codexy/skills/proof-driven-completion/SKILL.md",
    "plugins/codexy/skills/qa/SKILL.md",
];

const FAITHFUL_CASES: [(&str, &str); 3] = [
    (
        "재현 게이트가 통과되지 않아 현재 lane은 BLOCK 상태입니다.",
        "문제를 아직 재현하지 못해 수정을 시작할 수 없습니다.",
    ),
    (
        "packaged Sentinel gate가 PASS했고 handoff가 준비되었습니다.",
        "최종 검토를 통과해 결과를 전달할 준비가 됐습니다.",
    ),
    (
        "intake receipt 승인 후 lane을 시작했고 terminal receipt를 parent에 handoff했습니다.",
        "이슈 생성 전 확인을 마치고 작업을 시작했습니다. 종료 기록은 별도 증거에 보관했습니다.",
    ),
];

#[test]
fn representative_skills_share_plain_korean_response_guidance() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guidance = std::fs::read_to_string(root.join(SHARED_GUIDANCE))?;

    for heading in [
        "## User Summary",
        "## Machine-Readable Evidence",
        "## Protected Technical Text",
        "## Examples",
    ] {
        assert!(guidance.contains(heading), "missing {heading}");
    }

    for term in [
        "intake receipt",
        "terminal receipt",
        "handoff",
        "packaged",
        "gate",
        "lane",
    ] {
        assert!(guidance.contains(term), "missing internal term {term}");
    }

    for protected in [
        "code",
        "commands",
        "paths",
        "identifiers",
        "issue/PR numbers",
        "product names",
        "MUST/MUST NOT",
    ] {
        assert!(guidance.contains(protected), "missing protected text {protected}");
    }

    assert!(guidance.contains("MUST keep machine-readable evidence complete"));
    assert!(guidance.contains("MUST NOT expose internal workflow vocabulary"));
    assert!(guidance.contains("MUST explain it briefly in plain Korean"));

    for (before, after) in FAITHFUL_CASES {
        assert!(guidance.contains(before), "missing RED example: {before}");
        assert!(guidance.contains(after), "missing GREEN example: {after}");
    }

    for path in REPRESENTATIVE_SKILLS {
        let skill = std::fs::read_to_string(root.join(path))?;
        assert!(
            skill.contains("natural-korean-responses.md"),
            "{path} does not route Korean user-facing replies through shared guidance"
        );
    }

    let agent_prompt = std::fs::read_to_string(root.join("plugins/codexy/agents/openai.yaml"))?;
    let manifest = std::fs::read_to_string(root.join("plugins/codexy/.codex-plugin/plugin.json"))?;
    for prompt in [&agent_prompt, &manifest] {
        assert!(prompt.contains("When responding in Korean"));
        assert!(prompt.contains("plain, idiomatic Korean"));
        assert!(prompt.contains("machine-readable evidence separate"));
    }

    Ok(())
}
