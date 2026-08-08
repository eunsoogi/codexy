use std::path::Path;

use crate::paths::display_relative;

pub(super) fn require_all(
    path: &Path,
    text: &str,
    errors: &mut Vec<String>,
    requirement: &str,
    phrases: &[&str],
) {
    let lower = normalized_whitespace(text);
    for phrase in phrases {
        let phrase = normalized_whitespace(phrase);
        if !has_unweakened_required_clause(&lower, &phrase) {
            errors.push(format!(
                "{} {requirement}: missing `{phrase}`",
                display_relative(path)
            ));
        }
    }
}

pub(super) fn reject_all(
    path: &Path,
    text: &str,
    errors: &mut Vec<String>,
    requirement: &str,
    phrases: &[&str],
) {
    let lower = normalized_whitespace(text);
    for phrase in phrases {
        let phrase = normalized_whitespace(phrase);
        if lower.match_indices(&phrase).any(|(index, _)| {
            let before = &lower[..index];
            !appears_in_heading(before) && !has_invalid_prefix(before)
        }) {
            errors.push(format!(
                "{} {requirement}: forbidden `{phrase}`",
                display_relative(path)
            ));
        }
    }
}

fn has_unweakened_required_clause(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(index, _)| {
        let before = &text[..index];
        let after = text[index + phrase.len()..]
            .trim_start_matches([',', ':', ';', '-', '—'])
            .trim_start();
        !appears_in_heading(before) && !has_invalid_prefix(before) && !has_invalid_suffix(after)
    })
}

fn has_invalid_prefix(before: &str) -> bool {
    let section = before
        .rsplit("<markdown-heading>")
        .next()
        .unwrap_or_default();
    let clause = clause_prefix(section);
    section
        .trim_start()
        .starts_with("historical example </markdown-heading>")
        || clause.contains("historical example")
        || clause.contains("false that")
        || clause.starts_with("not required")
        || clause.starts_with("no longer required")
        || clause.trim_end().ends_with("it is not required that")
}

fn clause_prefix(section: &str) -> &str {
    let mut start = 0;
    for (index, character) in section.char_indices() {
        let after = section[index + character.len_utf8()..].trim_start();
        if character == ';'
            || (character == '.'
                && !after
                    .chars()
                    .next()
                    .is_some_and(|item| item.is_ascii_digit()))
        {
            start = index + character.len_utf8();
        }
    }
    &section[start..]
}

fn has_invalid_suffix(after: &str) -> bool {
    ["unless ", "except ", "only if ", "may ", "is not required"]
        .iter()
        .any(|marker| after.starts_with(marker))
}

fn appears_in_heading(before: &str) -> bool {
    before.rfind("<markdown-heading>") > before.rfind("</markdown-heading>")
}

fn normalized_whitespace(text: &str) -> String {
    let mut with_heading_boundaries = String::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            with_heading_boundaries.push_str(" <markdown-heading> ");
            with_heading_boundaries.push_str(trimmed.trim_start_matches('#').trim());
            with_heading_boundaries.push_str(" </markdown-heading> ");
        } else {
            with_heading_boundaries.push_str(line);
            with_heading_boundaries.push(' ');
        }
    }
    with_heading_boundaries
        .to_ascii_lowercase()
        .replace('`', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
