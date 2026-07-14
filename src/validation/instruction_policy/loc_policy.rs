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
        clauses(line).any(|clause| permits_in(clause, previous))
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
    exception_context && has_positive_permission(&words)
}

fn has_positive_permission(words: &[String]) -> bool {
    if has_exception_carve_out(words) {
        return true;
    }
    words
        .iter()
        .enumerate()
        .any(|(index, word)| match word.as_str() {
            "may" | "can" => words.get(index + 1).is_none_or(|next| next != "not"),
            "unless" => true,
            "must" => matches!(
                words.get(index + 1).map(String::as_str),
                Some("allow" | "exempt")
            ),
            word if is_passive_permission(word) => !passive_permission_is_negated(words, index),
            _ => false,
        })
}

fn is_passive_permission(word: &str) -> bool {
    matches!(word, "acceptable" | "allowed" | "authorized" | "permitted")
}

fn has_exception_carve_out(words: &[String]) -> bool {
    words.iter().any(|word| word == "except")
        || words
            .windows(2)
            .any(|pair| matches!(pair, [first, second] if first == "other" && second == "than"))
        || words
            .windows(2)
            .any(|pair| matches!(pair, [first, second] if first == "subject" && second == "to"))
        || words
            .windows(2)
            .any(|pair| matches!(pair, [first, second] if first == "provided" && second == "that"))
        || words
            .windows(2)
            .any(|pair| matches!(pair, [first, second] if first == "save" && second == "for"))
}

fn passive_permission_is_negated(words: &[String], index: usize) -> bool {
    let prefix = &words[..index];
    if prefix.last().is_some_and(|word| word == "not")
        || prefix.len() >= 2
            && prefix[prefix.len() - 2] == "not"
            && prefix[prefix.len() - 1] == "be"
    {
        return true;
    }
    prefix
        .iter()
        .rposition(|word| is_passive_permission(word))
        .is_some_and(|previous| {
            passive_permission_is_negated(words, previous)
                && words[previous + 1..index]
                    .iter()
                    .all(|word| matches!(word.as_str(), "or" | "nor" | "otherwise"))
        })
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
    clauses(text).any(|statement| {
        let statement = normalize(statement);
        statement.contains(&clause) && !negates_clause(&statement, &clause)
    })
}

fn clauses(text: &str) -> impl Iterator<Item = &str> {
    text.split(['.', ';', '!', '?'])
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
