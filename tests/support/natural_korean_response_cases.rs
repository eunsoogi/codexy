pub(super) const INTERNAL_TERMS: [&str; 6] = [
    "intake receipt",
    "terminal receipt",
    "handoff",
    "packaged",
    "gate",
    "lane",
];

struct ResponseCase {
    name: &'static str,
    user_summary: &'static str,
    machine_evidence: &'static str,
    protected: &'static [&'static str],
    valid: bool,
}

pub(super) fn assert_response_cases() {
    let cases = [
        ResponseCase {
            name: "orchestration",
            user_summary: "작업 범위와 담당자를 확인해 구현을 시작했습니다.",
            machine_evidence: r##"{"context":"orchestration","issue":"#460","rule":"MUST/MUST NOT"}"##,
            protected: &["#460", "MUST/MUST NOT"],
            valid: true,
        },
        ResponseCase {
            name: "debugging",
            user_summary: "문제를 재현했고 원인을 확인했습니다.",
            machine_evidence: r##"{"command":"cargo test --test suite_all","path":"plugins/codexy/skills/debugging/SKILL.md","identifier":"response_errors","product":"Codexy"}"##,
            protected: &[
                "cargo test --test suite_all",
                "plugins/codexy/skills/debugging/SKILL.md",
                "response_errors",
                "Codexy",
            ],
            valid: true,
        },
        ResponseCase {
            name: "completion",
            user_summary: "모든 검증을 통과해 결과를 전달할 준비가 됐습니다.",
            machine_evidence: r##"{"packaged":"codexy-sentinel","gate":"PASS","handoff":"ready","pr":"PR #462"}"##,
            protected: &["codexy-sentinel", "PASS", "PR #462"],
            valid: true,
        },
        ResponseCase {
            name: "receipt-heavy progress",
            user_summary: "이슈 생성 전 확인을 마치고 작업을 시작했습니다. 종료 기록은 별도 증거에 보관했습니다.",
            machine_evidence: r##"{"intake receipt":"approved","terminal receipt":"recorded","lane":"#460","handoff":"parent","packaged":"yes","gate":"PASS"}"##,
            protected: &["#460", "PASS"],
            valid: true,
        },
        ResponseCase {
            name: "leaky summary",
            user_summary: "intake receipt 승인 후 lane을 시작했습니다.",
            machine_evidence: r##"{"intake receipt":"approved","lane":"#460"}"##,
            protected: &["#460"],
            valid: false,
        },
        ResponseCase {
            name: "translated protected literals",
            user_summary: "문제를 재현했고 원인을 확인했습니다.",
            machine_evidence: r##"{"command":"카고 테스트","path":"디버깅 스킬","product":"코덱시"}"##,
            protected: &[
                "cargo test --test suite_all",
                "plugins/codexy/skills/debugging/SKILL.md",
                "Codexy",
            ],
            valid: false,
        },
        ResponseCase {
            name: "informal tone",
            user_summary: "수정 끝났어.",
            machine_evidence: r##"{"issue":"#460"}"##,
            protected: &["#460"],
            valid: false,
        },
    ];

    for case in cases {
        let errors = response_errors(&case);
        assert_eq!(errors.is_empty(), case.valid, "{}: {errors:?}", case.name);
    }
}

fn response_errors(case: &ResponseCase) -> Vec<String> {
    let mut errors = Vec::new();
    for term in INTERNAL_TERMS {
        if case.user_summary.contains(term) {
            errors.push(format!("internal term leaked: {term}"));
        }
    }
    for sentence in case
        .user_summary
        .split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if !["습니다", "입니다", "됩니다"]
            .iter()
            .any(|ending| sentence.ends_with(ending))
        {
            errors.push(format!("non-honorific sentence: {sentence}"));
        }
    }
    if !(case.machine_evidence.starts_with('{') && case.machine_evidence.ends_with('}')) {
        errors.push("machine evidence is not separate structured data".into());
    }
    for literal in case.protected {
        if !case.machine_evidence.contains(literal) {
            errors.push(format!("protected literal changed: {literal}"));
        }
    }
    errors
}
