use std::path::Path;

use crate::paths::display_relative;

mod overage;
mod surfaces;
#[cfg(test)]
mod tests;
use surfaces::{
    EXCEPTION_PROHIBITION, GOVERNED_AGENT_ROLES, GOVERNED_SKILLS, UNCONDITIONAL_CONTRACT,
};

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    let governed_skill = GOVERNED_SKILLS.iter().any(|skill| path.ends_with(skill));
    let governed_agent_role = GOVERNED_AGENT_ROLES.iter().any(|role| path.ends_with(role));
    let governed_root_agents = surfaces::is_governed_root_agents(path);
    if !governed_skill && !governed_agent_role && !governed_root_agents {
        return;
    }
    if governed_skill && !contains_clause(text, UNCONDITIONAL_CONTRACT) {
        errors.push(format!(
            "{} LOC exception policy contract failed: missing unconditional governed 250 LOC clause",
            display_relative(path)
        ));
    }
    if !governed_root_agents && !contains_clause(text, EXCEPTION_PROHIBITION) {
        errors.push(format!(
            "{} LOC exception policy contract failed: missing LOC exception prohibition",
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
    let mut exception_section_level = None;
    let mut previous_exception_heading = false;
    text.lines().map(str::trim).any(|line| {
        if line.is_empty() {
            return false;
        }
        let exception_heading = is_exception_heading(line);
        if let Some(level) = markdown_heading_level(line) {
            if exception_section_level.is_some_and(|section| level <= section) {
                exception_section_level = None;
            }
            if exception_heading {
                exception_section_level = Some(level);
            }
        }
        let inherited_context = exception_section_level.is_some() || previous_exception_heading;
        previous_exception_heading = exception_heading;
        clauses(line).any(|clause| permits_in(clause, inherited_context))
    })
}

fn markdown_heading_level(text: &str) -> Option<usize> {
    let level = text.bytes().take_while(|byte| *byte == b'#').count();
    (1..=6).contains(&level).then_some(level).filter(|level| {
        text.as_bytes()
            .get(*level)
            .is_some_and(u8::is_ascii_whitespace)
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
        .any(|word| matches!(word.as_str(), "loc" | "250" | "governed"))
        || words
            .windows(2)
            .any(|pair| matches!(pair, [first, second] if first == "large" && second == "file"));
    let overage_authorization = authorizes_loc_overage(&words, loc_context);
    let exception_context = inherited_context
        || (exception_term && loc_context)
        || overage_authorization
        || words.iter().any(|word| {
            matches!(
                word.as_str(),
                "waiver" | "waivers" | "exempt" | "exempted" | "exemption"
            )
        });
    exception_context && (overage_authorization || has_positive_permission(&words))
}

fn authorizes_loc_overage(words: &[String], loc_context: bool) -> bool {
    loc_context
        && words.iter().enumerate().any(|(index, word)| {
            matches!(
                word.as_str(),
                "exceed" | "exceeded" | "exceeding" | "exceeds"
            ) && (words[index + 1..]
                .iter()
                .position(|word| word == "without")
                .is_some_and(|without| {
                    words[index + 1 + without..].iter().any(|word| {
                        matches!(word.as_str(), "justification" | "rationale" | "reason")
                    })
                })
                || index > 0 && matches!(words[index - 1].as_str(), "may" | "can")
                || has_governing_passive_permission(words, index))
                && !overage::is_negated(words, index)
        })
}

fn has_governing_passive_permission(words: &[String], exceed: usize) -> bool {
    let Some(subject) = words[..exceed]
        .windows(3)
        .rposition(|phrase| matches!(phrase, [governed, subject, verb] if governed == "governed" && matches!(subject.as_str(), "file" | "files") && matches!(verb.as_str(), "is" | "are")))
    else {
        return false;
    };
    words[subject + 3..exceed]
        .iter()
        .rposition(|word| is_passive_permission(word))
        .is_some_and(|permission| !passive_permission_is_negated(words, subject + 3 + permission))
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
    matches!(
        word,
        "acceptable"
            | "approve"
            | "approved"
            | "allowed"
            | "authorized"
            | "exempt"
            | "exempted"
            | "permitted"
            | "waive"
            | "waived"
    )
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
