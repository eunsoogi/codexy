use std::collections::BTreeSet;

#[path = "structured_contract.rs"]
mod structured_contract;
use structured_contract::{Contract, Modality, Rule};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const GUIDANCE: &str =
    "plugins/codexy/skills/codex-orchestration/references/plain-language-user-replies.md";

const OPAQUE_TERMS: [&str; 8] = [
    "Sentinel verdict",
    "terminal handoff",
    "delta",
    "heartbeat route",
    "gate",
    "lane",
    "packaged",
    "faithful RED coverage",
];

const MAPPINGS: [(&str, &str, &str); 8] = [
    ("Sentinel verdict", "final review result", "최종 검토 결과"),
    ("terminal handoff", "final status and next action", "최종 상태와 다음 조치"),
    ("delta", "changed fact", "달라진 사실"),
    ("heartbeat route", "scheduled read-only check", "예약된 읽기 전용 점검"),
    ("gate", "required check", "필수 확인"),
    ("lane", "this issue", "이 이슈"),
    ("packaged", "bundled with Codexy", "Codexy에 포함된"),
    ("faithful RED coverage", "original-failure test", "원래 실패를 보여 주는 테스트"),
];

const SAFE_GATE_EXAMPLES: [(&str, &str); 2] = [
    (
        "The heartbeat route is waiting on the final gate.",
        "A scheduled read-only check is waiting for the final required check.",
    ),
    (
        "heartbeat route가 마지막 gate를 기다리고 있습니다.",
        "예약된 읽기 전용 점검이 마지막 필수 확인을 기다리고 있습니다.",
    ),
];

const ROUTED_SURFACES: [&str; 4] = [
    "plugins/codexy/skills/codex-orchestration/SKILL.md",
    "plugins/codexy/skills/debugging/SKILL.md",
    "plugins/codexy/skills/proof-driven-completion/SKILL.md",
    "plugins/codexy/skills/qa/SKILL.md",
];

const GUIDANCE_RULES: &[Rule] = &[
    Rule::new(
        "plain-language.summary.replace-internal-terms",
        "you",
        Modality::Required,
        &["replace"],
        &["unnecessary internal workflow terms", "concrete event"],
    )
    .under_heading("user summary"),
    Rule::new(
        "plain-language.summary.next-action-evidence",
        "you",
        Modality::Required,
        &["keep"],
        &["next-action claims", "faithful", "verified evidence"],
    )
    .under_heading("user summary"),
    Rule::new(
        "plain-language.summary.no-unexplained-terms",
        "you",
        Modality::Prohibited,
        &["expose"],
        &["unexplained internal term"],
    )
    .under_heading("user summary"),
    Rule::new(
        "plain-language.english.ordinary-language",
        "you",
        Modality::Required,
        &["prefer"],
        &["short", "direct sentences", "ordinary workflow language"],
    )
    .under_heading("english"),
    Rule::new(
        "plain-language.korean.natural-language",
        "you",
        Modality::Required,
        &["use"],
        &["natural Korean word order", "context-appropriate honorific tone"],
    )
    .under_heading("korean"),
    Rule::new(
        "plain-language.evidence.unchanged",
        "exact schema names, validator fields, commands, identifiers, and machine-readable evidence",
        Modality::Required,
        &["remain"],
        &["complete", "unchanged"],
    )
    .under_heading("protected evidence"),
    Rule::new(
        "plain-language.evidence.separate",
        "you",
        Modality::Required,
        &["keep"],
        &["protected evidence", "separate", "user summary"],
    )
    .under_heading("protected evidence"),
    Rule::new(
        "plain-language.contracts.no-rename",
        "it",
        Modality::Prohibited,
        &["rename"],
        &["internal contracts"],
    )
    .under_heading("protected evidence"),
];

#[test]
fn shared_guidance_covers_plain_english_and_korean_replies() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let guidance = std::fs::read_to_string(root.join(GUIDANCE))?;

    structured_contract::assert_rules(
        &Contract::markdown_for_subject(&guidance, "you"),
        GUIDANCE_RULES,
    );
    assert_eq!(
        protected_evidence_literals(&guidance),
        BTreeSet::from(["MUST/MUST NOT"]),
        "protected technical literals changed"
    );
    assert_eq!(
        gate_examples(&guidance),
        BTreeSet::from(SAFE_GATE_EXAMPLES),
        "English and Korean gate examples must stay action-neutral"
    );

    let mappings = markdown_table_pairs(&guidance);
    let mut expected_mappings = BTreeSet::new();
    for (term, english, korean) in MAPPINGS {
        expected_mappings.insert((term, english));
        expected_mappings.insert((term, korean));
    }
    assert_eq!(mappings, expected_mappings, "exact bilingual mappings changed");

    for path in ROUTED_SURFACES {
        let text = std::fs::read_to_string(root.join(path))?;
        let target = if path.ends_with("codex-orchestration/SKILL.md") {
            "references/plain-language-user-replies.md"
        } else {
            "../codex-orchestration/references/plain-language-user-replies.md"
        };
        assert_eq!(
            reference_targets(&text),
            BTreeSet::from([target.to_owned()]),
            "plain-language route changed in {path}"
        );
    }

    Ok(())
}

#[test]
fn unexplained_terms_fail_while_concrete_or_explained_summaries_pass() {
    for (term, english, korean) in MAPPINGS {
        for summary in [format!("{term}: ready"), format!("{term}(noise): ready")] {
            assert!(!plain_summary(&summary), "unexplained wording passed: {summary}");
        }
        for summary in [english, korean, &format!("{term} ({english})"), &format!("{term}({korean})")] {
            assert!(plain_summary(summary), "plain or explained wording failed: {summary}");
        }
    }

    for summary in [
        "The final review passed, so the result is ready to share.",
        "A scheduled read-only check is waiting for the final required check.",
        "최종 검토를 통과해 결과를 전달할 준비가 됐습니다.",
        "실패를 먼저 보여 주는 테스트를 추가한 뒤 수정했습니다.",
    ] {
        assert!(plain_summary(summary), "plain or explained wording failed: {summary}");
    }
}

fn plain_summary(summary: &str) -> bool {
    MAPPINGS.iter().all(|(term, english, korean)| {
        summary.match_indices(term).all(|(index, _)| {
            let suffix = &summary[index + term.len()..];
            let explanation = suffix
                .strip_prefix(" (")
                .or_else(|| suffix.strip_prefix('('))
                .and_then(|value| value.split_once(')'))
                .map(|(value, _)| value);
            explanation.is_some_and(|value| value == *english || value == *korean)
        })
    })
}

fn markdown_table_pairs(text: &str) -> BTreeSet<(&str, &str)> {
    text.lines()
        .filter(|line| line.starts_with('|') && !line.starts_with("| ---"))
        .filter_map(|line| {
            let cells: Vec<_> = line.trim_matches('|').split('|').map(str::trim).collect();
            let term = cells.first()?.trim_matches('`');
            (cells.len() == 2 && OPAQUE_TERMS.iter().any(|candidate| candidate == &term))
                .then(|| (term, cells[1]))
        })
        .collect()
}

fn reference_targets(text: &str) -> BTreeSet<String> {
    text.split(|character: char| {
        character.is_whitespace()
            || matches!(character, '`' | '(' | ')' | '[' | ']' | ',' | ':')
    })
        .filter(|value| value.ends_with("plain-language-user-replies.md"))
        .map(str::to_owned)
        .collect()
}

fn protected_evidence_literals(text: &str) -> BTreeSet<&str> {
    let section = text
        .split_once("## Protected Evidence")
        .map(|(_, tail)| tail)
        .unwrap_or_default()
        .split_once("\n## ")
        .map(|(body, _)| body)
        .unwrap_or_default();
    section
        .split('`')
        .enumerate()
        .filter_map(|(index, literal)| (index % 2 == 1).then_some(literal))
        .collect()
}

fn gate_examples(text: &str) -> BTreeSet<(&str, &str)> {
    text.lines()
        .filter(|line| line.starts_with('|') && line.contains("heartbeat route"))
        .filter_map(|line| {
            let cells: Vec<_> = line.trim_matches('|').split('|').map(str::trim).collect();
            (cells.len() == 2 && !cells[0].starts_with('`')).then(|| (cells[0], cells[1]))
        })
        .collect()
}
