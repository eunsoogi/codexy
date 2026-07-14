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
        let previous = index
            .checked_sub(1)
            .and_then(|previous| lines.get(previous))
            .is_some_and(|previous| is_exception_heading(previous));
        permits_in(line, previous)
    })
}

fn is_exception_heading(text: &str) -> bool {
    let words = words(text);
    words
        .iter()
        .any(|word| matches!(word.as_str(), "exception" | "exceptions"))
        && words.iter().any(|word| word == "loc")
}

fn permits_in(text: &str, inherited_context: bool) -> bool {
    let words = words(text);
    let negative = words.windows(2).any(|pair| {
        matches!(
            (pair[0].as_str(), pair[1].as_str()),
            ("must", "not") | ("may", "not")
        )
    });
    let exception_term = words
        .iter()
        .any(|word| matches!(word.as_str(), "exception" | "exceptions"));
    let loc_context = words
        .iter()
        .any(|word| matches!(word.as_str(), "loc" | "250" | "governed"));
    let exception_context = inherited_context
        || (exception_term && loc_context)
        || words
            .iter()
            .any(|word| matches!(word.as_str(), "waiver" | "waivers" | "exempt" | "exemption"));
    let permission = words
        .iter()
        .any(|word| matches!(word.as_str(), "may" | "can" | "unless"))
        || words.windows(2).any(|pair| {
            matches!(
                (pair[0].as_str(), pair[1].as_str()),
                ("must", "allow") | ("must", "exempt")
            )
        });
    exception_context && permission && !negative
}

fn words(text: &str) -> Vec<String> {
    text.to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphabetic() && !character.is_ascii_digit())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

fn contains_clause(text: &str, clause: &str) -> bool {
    let clause = normalize(clause);
    text.split(['.', '!', '?']).any(|statement| {
        let statement = normalize(statement);
        statement.contains(&clause) && !negates_clause(&statement, &clause)
    })
}

fn normalize(text: &str) -> String {
    text.to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn negates_clause(statement: &str, clause: &str) -> bool {
    statement
        .split_once(clause)
        .is_some_and(|(prefix, _)| prefix.contains("must not") || prefix.contains("may not"))
}
