use std::path::Path;

use crate::paths::display_relative;

const GOVERNED_SKILLS: &[&str] = &[
    "skills/git-workflow/SKILL.md",
    "skills/plugin-marketplace-prep/SKILL.md",
    "skills/proof-driven-completion/SKILL.md",
    "skills/refactoring/SKILL.md",
];
const UNCONDITIONAL_CONTRACT: &str = "every governed file MUST stay at or below 250 LOC";
const EXCEPTION_PROHIBITION: &str = "MUST NOT use or authorize LOC exceptions";

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    if !GOVERNED_SKILLS.iter().any(|skill| path.ends_with(skill)) {
        return;
    }
    if !contains_clause(text, UNCONDITIONAL_CONTRACT)
        || !contains_clause(text, EXCEPTION_PROHIBITION)
    {
        errors.push(format!(
            "{} LOC exception policy contract failed: missing unconditional governed 250 LOC clause",
            display_relative(path)
        ));
    }
    if permits_exception_allowance(text) {
        errors.push(format!(
            "{} LOC exception policy contract failed: must not allow LOC exceptions",
            display_relative(path)
        ));
    }
}

fn permits_exception_allowance(text: &str) -> bool {
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    lines.iter().enumerate().any(|(index, line)| {
        let next = lines.get(index + 1).copied().unwrap_or_default();
        permits_in(&format!("{line} {next}"))
    })
}

fn permits_in(text: &str) -> bool {
    let normalized = text.to_ascii_lowercase();
    let words = normalized
        .split(|character: char| !character.is_ascii_alphabetic() && !character.is_ascii_digit())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let negative = words
        .windows(2)
        .any(|pair| matches!(pair, ["must", "not"] | ["may", "not"]));
    let exception_term = words
        .iter()
        .any(|word| matches!(*word, "exception" | "exceptions"));
    let loc_context = words
        .iter()
        .any(|word| matches!(*word, "loc" | "250" | "governed"));
    let exception_context = (exception_term && loc_context)
        || words
            .iter()
            .any(|word| matches!(*word, "waiver" | "waivers" | "exempt" | "exemption"));
    let permission = words
        .iter()
        .any(|word| matches!(*word, "may" | "can" | "unless"))
        || words
            .windows(2)
            .any(|pair| matches!(pair, ["must", "allow"] | ["must", "exempt"]));
    exception_context && permission && !negative
}

fn contains_clause(text: &str, clause: &str) -> bool {
    text.to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .contains(
            &clause
                .to_ascii_lowercase()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
        )
}
