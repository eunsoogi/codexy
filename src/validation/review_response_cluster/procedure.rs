use std::path::Path;

use crate::paths::display_relative;
use unicode_normalization::UnicodeNormalization;

const REFERENCE_PATH: &str = "skills/git-workflow/references/review-response-clusters.md";
const HEADING: &str = "## Required Procedure";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Obligation {
    ReceiptCreate,
    ReceiptValidate,
    CaseExceptionProhibition,
    ReopenEvidenceRestriction,
    FinalReceiptValidate,
}

impl Obligation {
    const ALL: [Self; 5] = [
        Self::ReceiptCreate,
        Self::ReceiptValidate,
        Self::CaseExceptionProhibition,
        Self::ReopenEvidenceRestriction,
        Self::FinalReceiptValidate,
    ];

    fn parse(id: &str) -> Option<Self> {
        match id {
            "receipt-create" => Some(Self::ReceiptCreate),
            "receipt-validate" => Some(Self::ReceiptValidate),
            "case-exception-prohibition" => Some(Self::CaseExceptionProhibition),
            "reopen-evidence-restriction" => Some(Self::ReopenEvidenceRestriction),
            "final-receipt-validate" => Some(Self::FinalReceiptValidate),
            _ => None,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::ReceiptCreate => "receipt-create",
            Self::ReceiptValidate => "receipt-validate",
            Self::CaseExceptionProhibition => "case-exception-prohibition",
            Self::ReopenEvidenceRestriction => "reopen-evidence-restriction",
            Self::FinalReceiptValidate => "final-receipt-validate",
        }
    }

    fn is_prohibition(self) -> bool {
        matches!(
            self,
            Self::CaseExceptionProhibition | Self::ReopenEvidenceRestriction
        )
    }

    fn clause(self) -> &'static str {
        match self {
            Self::ReceiptCreate => {
                "before editing actionable review feedback must create one typed json receipt"
            }
            Self::ReceiptValidate => {
                "before implementation must validate that exact receipt file with scripts/validate-plugin-config --check-review-response-cluster --review-response-cluster-file receipt.json"
            }
            Self::CaseExceptionProhibition => {
                "during repair must not accept a case-specific exception as structural evidence"
            }
            Self::ReopenEvidenceRestriction => {
                "non-reopened receipt states must not include reopen evidence"
            }
            Self::FinalReceiptValidate => {
                "after addressing feedback and before push or handoff must set the receipt state to repaired or reopened and validate that exact final-state file with scripts/validate-plugin-config --check-review-response-cluster --review-response-cluster-file receipt.json"
            }
        }
    }
}

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    if !path.ends_with(REFERENCE_PATH) {
        return;
    }
    let mut in_procedure = false;
    let mut fence = None;
    let mut seen = std::collections::BTreeSet::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some((marker, length)) = fence {
            if closes_fence(line, marker, length) {
                fence = None;
            }
            continue;
        }
        if let Some(opening) = opening_fence(line) {
            fence = Some(opening);
            continue;
        }
        if line == HEADING {
            in_procedure = true;
            continue;
        }
        if in_procedure && line.starts_with("## ") {
            break;
        }
        if !in_procedure {
            continue;
        }
        let Some(content) = numbered_step_content(line) else {
            continue;
        };
        let Some((id, obligation_text)) = obligation_parts(content) else {
            errors.push(format!(
                "{} review procedure numbered step must start with a stable obligation ID",
                display_relative(path)
            ));
            continue;
        };
        let Some(obligation) = Obligation::parse(id) else {
            errors.push(format!(
                "{} review procedure contains unknown obligation ID [{id}]",
                display_relative(path)
            ));
            continue;
        };
        if !seen.insert(obligation) {
            errors.push(format!(
                "{} review procedure contains duplicate obligation ID [{}]",
                display_relative(path),
                obligation.id()
            ));
        }
        let has_must_not = has_must_not(obligation_text);
        let valid_polarity = if obligation.is_prohibition() {
            has_must_not
        } else {
            has_must(obligation_text) && !has_must_not
        };
        if !valid_polarity {
            let required = if obligation.is_prohibition() {
                "MUST NOT"
            } else {
                "MUST"
            };
            errors.push(format!(
                "{} review procedure obligation [{}] must use {required}",
                display_relative(path),
                obligation.id()
            ));
        }
        let normalized_clause = normalize_clause(obligation_text);
        if normalized_clause != obligation.clause() {
            errors.push(format!(
                "{} review procedure obligation [{}] must retain its required action semantics (expected {}, got {})",
                display_relative(path),
                obligation.id(),
                obligation.clause(),
                normalized_clause
            ));
        }
    }
    for obligation in Obligation::ALL {
        if !seen.contains(&obligation) {
            errors.push(format!(
                "{} review procedure is missing obligation ID [{}]",
                display_relative(path),
                obligation.id()
            ));
        }
    }
    if seen.is_empty() {
        errors.push(format!(
            "{} review procedure must include the complete typed obligation catalog",
            display_relative(path)
        ));
    }
}

fn opening_fence(line: &str) -> Option<(char, usize)> {
    let marker = line.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let length = line
        .chars()
        .take_while(|character| *character == marker)
        .count();
    (length >= 3).then_some((marker, length))
}

fn closes_fence(line: &str, marker: char, opening_length: usize) -> bool {
    let length = line
        .chars()
        .take_while(|character| *character == marker)
        .count();
    length >= opening_length && line[length..].trim().is_empty()
}

fn numbered_step_content(line: &str) -> Option<&str> {
    let (prefix, content) = line.split_once(". ")?;
    if prefix.is_empty() || !prefix.bytes().all(|byte| byte.is_ascii_digit()) || content.is_empty()
    {
        return None;
    }
    Some(content)
}

fn obligation_parts(content: &str) -> Option<(&str, &str)> {
    let remainder = content.strip_prefix('[')?;
    let (id, text) = remainder.split_once("] ")?;
    if id.is_empty() || text.is_empty() {
        return None;
    }
    Some((id, text))
}

fn has_must(text: &str) -> bool {
    words(text).any(|word| word == "MUST")
}

fn has_must_not(text: &str) -> bool {
    let words = words(text).collect::<Vec<_>>();
    words.windows(2).any(|pair| pair == ["MUST", "NOT"])
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|character: char| !character.is_ascii_alphabetic())
        .filter(|word| !word.is_empty())
}

fn normalize_clause(text: &str) -> String {
    let normalized = text
        .nfkc()
        .filter(|character| !matches!(character, '`' | ','))
        .collect::<String>();
    normalized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(['.', ':'])
        .to_lowercase()
}
