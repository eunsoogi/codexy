#[path = "structured_contract.rs"]
mod structured_contract;
#[path = "structured_contract_artifacts.rs"]
mod structured_contract_artifacts;
#[path = "support/natural_korean_response_cases.rs"]
mod response_cases;

use std::{collections::BTreeSet, path::Path};

use structured_contract::{Contract, Modality, Rule};

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

const GUIDANCE_RULES: &[Rule] = &[
    Rule::new(
        "korean.summary.no-internal-vocabulary",
        "user summary",
        Modality::Prohibited,
        &["expose"],
        &[],
    )
    .under_heading("user summary"),
    Rule::new(
        "korean.summary.explain-essential-term",
        "essential internal terms",
        Modality::Required,
        &["receive"],
        &["explanation", "plain Korean"],
    )
    .under_heading("user summary"),
    Rule::new(
        "korean.summary.honorific-tone",
        "user summary",
        Modality::Required,
        &["use"],
        &["context-appropriate honorific tone"],
    )
    .under_heading("user summary"),
    Rule::new(
        "korean.evidence.complete",
        "machine-readable evidence",
        Modality::Required,
        &["remain"],
        &["complete", "unchanged"],
    )
    .under_heading("machine-readable evidence"),
    Rule::new(
        "korean.technical-text.preserve",
        "you",
        Modality::Required,
        &["preserve"],
        &["code", "commands", "paths", "identifiers", "issue/PR numbers", "product names"],
    )
    .under_heading("protected technical text"),
];

const PROMPT_RULES: &[Rule] = &[
    Rule::new(
        "korean.prompt.idiomatic",
        "you",
        Modality::Required,
        &["use"],
        &["plain", "idiomatic Korean"],
    ),
    Rule::new(
        "korean.prompt.separate-evidence",
        "you",
        Modality::Required,
        &["keep"],
        &["machine-readable evidence", "separate", "user summary"],
    ),
];

#[test]
fn representative_skills_share_plain_korean_response_guidance() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guidance = std::fs::read_to_string(root.join(SHARED_GUIDANCE))?;

    structured_contract::assert_rules(
        &Contract::markdown_for_subject(&guidance, "you"),
        GUIDANCE_RULES,
    );
    let literals = reference_targets(&guidance);
    for term in response_cases::INTERNAL_TERMS {
        assert!(literals.iter().any(|literal| *literal == term), "missing internal term: {term}");
    }
    assert!(literals.iter().any(|literal| *literal == "MUST/MUST NOT"));
    let examples = markdown_table_pairs(&guidance);
    for pair in FAITHFUL_CASES {
        assert!(examples.iter().any(|example| *example == pair), "missing response example: {pair:?}");
    }

    for path in REPRESENTATIVE_SKILLS {
        let skill = std::fs::read_to_string(root.join(path))?;
        let expected = if path.ends_with("codex-orchestration/SKILL.md") {
            "references/natural-korean-responses.md"
        } else {
            "../codex-orchestration/references/natural-korean-responses.md"
        };
        assert!(
            reference_targets(&skill).iter().any(|target| *target == expected),
            "missing route in {path}"
        );
    }

    let agent_prompt = std::fs::read_to_string(root.join("plugins/codexy/agents/openai.yaml"))?;
    let manifest = std::fs::read_to_string(root.join("plugins/codexy/.codex-plugin/plugin.json"))?;
    let prompt = structured_contract_artifacts::Prompt::parse(&agent_prompt)?;
    structured_contract::assert_rules(
        &Contract::markdown_for_subject(prompt.default_prompt(), "you"),
        PROMPT_RULES,
    );
    let manifest: serde_json::Value = serde_json::from_str(&manifest)?;
    let manifest_prompt = manifest["interface"]["defaultPrompt"]
        .as_array()
        .ok_or("manifest defaultPrompt")?
        .iter()
        .map(|line| line.as_str().ok_or("manifest prompt line"))
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    structured_contract::assert_rules(
        &Contract::markdown_for_subject(&manifest_prompt, "you"),
        PROMPT_RULES,
    );

    response_cases::assert_response_cases();

    Ok(())
}

fn markdown_table_pairs(text: &str) -> BTreeSet<(&str, &str)> {
    text.lines()
        .filter(|line| line.starts_with('|') && !line.starts_with("| ---"))
        .filter_map(|line| {
            let cells: Vec<_> = line.trim_matches('|').split('|').map(str::trim).collect();
            (cells.len() == 2 && cells[0] != "Avoid").then(|| (cells[0], cells[1]))
        })
        .collect()
}

fn reference_targets(text: &str) -> BTreeSet<&str> {
    let mut targets = BTreeSet::new();
    for (index, part) in text.split('`').enumerate() {
        if index % 2 == 1 {
            targets.insert(part);
        }
    }
    for part in text.split("](").skip(1) {
        if let Some((target, _)) = part.split_once(')') {
            targets.insert(target);
        }
    }
    targets
}
